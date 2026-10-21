"""Candidate validation and handoff for the Codex native packer (numpy only; PIL optional for the atlas check).

Port of the checks in experiments/claude/nhl21/prepare_candidate_nhl21.py. The handoff layout is the one Codex's
`experiments/codex/nhl21_package/build.py` consumes: `artifacts.{native_mesh_json, native_mesh_sha256, atlas_png,
atlas_sha256}` + `inputs.{name: {path, sha256}}` (each verified by the packer) + status / blocking_problems / gates.
Nothing here is a native package; G1/G4 stay Codex's, G3/G5 stay open until measured/tested.
"""
import numpy as np
from . import geometry as G, face_quality as FQ, exchange as X
from .eye_fit import sphere_fit

HANDOFF_SCHEMA = 'claude-phase3-nhl21-candidate/1'


def validate_candidate(cand, ex, eyes_may_move=False, reference_head=None):
    """Structural checks of a candidate exchange against the untouched donor exchange. Returns (checks, problems, per_submesh)."""
    checks = {}; problems = []; per = {}

    def check(name, ok, detail=None):
        checks[name] = {'ok': bool(ok), **({'detail': detail} if detail is not None else {})}
        if not ok: problems.append(name)
    check('donor_sha256_equal_exchange', cand.get('donor_sha256') == ex['donor_sha256'])
    for k in ('schema', 'profile_id', 'coordinate_system', 'layout', 'rules', 'title'):
        check(f'top_level_{k}_equal', cand.get(k) == ex.get(k))
    names = [m['name'] for m in cand['submeshes']]; check('submesh_names_and_order', names == [m['name'] for m in ex['submeshes']], names)
    for m, v in zip(cand['submeshes'], ex['submeshes']):
        d = {'vertices': len(m['positions']), 'triangles': len(m['triangles'])}
        for k in ('original_positions', 'triangles', 'uv', 'normals', 'tangents', 'streams', 'format', 'role', 'native_fields', 'connected_components'):
            d[f'{k}_equal'] = m.get(k) == v.get(k); check(f'{m["name"]}:{k}_equal', d[f'{k}_equal'])
        P = np.array(m['positions']); op = np.array(v['original_positions'])
        d['positions_finite'] = bool(np.isfinite(P).all()); check(f'{m["name"]}:positions_finite', d['positions_finite'])
        d['float32_roundtrip_max_abs_m'] = float(np.abs(P.astype(np.float32).astype(np.float64) - P).max())
        d['max_move_vs_donor_mm'] = float(np.linalg.norm(P - op, axis=1).max() * 1000)
        d['bounds_min'] = P.min(0).tolist(); d['bounds_max'] = P.max(0).tolist(); d['donor_bounds_min'] = op.min(0).tolist(); d['donor_bounds_max'] = op.max(0).tolist()
        if m['name'] == X.HEAD:
            Pb = G.native_to_blender(P); Ob = G.native_to_blender(op); T = np.array(m['triangles']); dup = G.seam_groups(Ob); adj = G.adjacency(len(Ob), T, dup)
            loop = G.bottom_loop(Ob, T); frozen = G.grow(loop, adj, 2)
            d['frozen_attachment'] = {'bottom_loop_vertices': int(len(loop)), 'frozen_vertices_loop_plus_2_rings': int(len(frozen)), 'max_error_m': float(np.abs(Pb[frozen] - Ob[frozen]).max())}
            check('head:attachment_loop_plus_2_rings_exact', d['frozen_attachment']['max_error_m'] == 0.0)
            d['seam_max_gap_m'] = max((float(np.linalg.norm(Pb[ids] - Pb[ids[0]], axis=1).max()) for ids in dup), default=0.0); check('head:seam_duplicates_coincident', d['seam_max_gap_m'] == 0.0)
            d['crown_z'] = float(Pb[:, 2].max()); d['donor_crown_z'] = float(Ob[:, 2].max()); check('head:crown_within_1mm', abs(d['crown_z'] - d['donor_crown_z']) < 1e-3, {'delta_mm': (d['crown_z'] - d['donor_crown_z']) * 1000})
            q = G.triangle_quality(Ob, Pb, T); d['triangle_quality_vs_donor'] = q; check('head:no_collapsed_triangles_vs_donor', q['new_collapsed_triangles'] == 0)
            ma = FQ.face_metrics(Pb, T, dup); md = FQ.face_metrics(Ob, T, dup)
            d['face_metrics'] = {s: {'inverted_local': int(mm['inverted_local'].sum()), 'folded_edge_gt150': int(mm['folded_edge'].sum()), 'sliver_lt3deg': int(mm['sliver'].sum()), 'min_area_mm2': float(mm['area'].min() * 1e6), 'min_angle_deg': float(mm['min_angle_deg'].min())} for s, mm in (('donor', md), ('candidate', ma))}
            if reference_head is not None: check('head_positions_equal_reference_head', m['positions'] == reference_head)
        elif 'eyes' in m['name']:
            d['positions_equal_donor'] = bool(np.array_equal(P, op))
            if not eyes_may_move: check(f'{m["name"]}:positions_untouched', d['positions_equal_donor'])
            d['components'] = {}
            for ci, c in enumerate(v['connected_components']):
                ids = np.array(c['vertex_ids']); Ob = G.native_to_blender(op[ids]); Pb = G.native_to_blender(P[ids])
                co, ro, _ = sphere_fit(Ob); cn, rn, _ = sphere_fit(Pb); sc = rn / ro; recon = (Ob - co) * sc + cn
                d['components'][ci] = {'side': 'L' if co[0] > 0 else 'R', 'uniform_scale': float(sc), 'translation_m': (cn - co).tolist(), 'reconstruction_rms_mm': float(np.sqrt(((recon - Pb) ** 2).sum(1).mean()) * 1000), 'sphere_centre_m': cn.tolist(), 'sphere_radius_mm': float(rn * 1000), 'donor_sphere_radius_mm': float(ro * 1000)}
                if eyes_may_move: check(f'{m["name"]}:component{ci}_uniform_scale_plus_translation', d['components'][ci]['reconstruction_rms_mm'] < 1e-6 and sc > 0)
        else:
            d['positions_equal_donor'] = bool(np.array_equal(P, op)); check(f'{m["name"]}:positions_untouched', d['positions_equal_donor'])
        per[m['name']] = d
    return checks, problems, per


def atlas_check(atlas_path, expected=(1024, 1024)):
    try:
        from PIL import Image
    except ImportError:
        return {'size': None, 'mode': None, 'ok': None, 'note': 'PIL unavailable in this interpreter; size check skipped (unknown)'}
    im = Image.open(atlas_path); return {'size': list(im.size), 'mode': im.mode, 'ok': tuple(im.size) == tuple(expected)}


def handoff(job_id, fixture, player, donor, status, problems, artifacts, inputs, validation_json, evidence, geometry_classification, eyes, masks_summary, what_this_is, needs_from_codex, limitations, gates, loss_report=None, provenance=None):
    return {'schema': HANDOFF_SCHEMA, 'based_on': 'claude-phase2-candidate/1 (same artifact/inputs/validation layout) + Codex nhl21-phase3-handoff/1 exchange schema; produced by the reusable worker slice (nhl-head-job/1 aligned)',
            'job_id': job_id, 'fixture': fixture, 'player': player, 'donor': donor, 'status': status, 'blocking_problems': problems, 'artifacts': artifacts, 'inputs': inputs, 'validation_json': validation_json,
            'evidence': evidence, 'geometry_classification': geometry_classification, 'eyes': eyes, 'masks': masks_summary, 'what_this_is': what_this_is, 'needs_from_codex': needs_from_codex,
            'limitations': limitations, 'gates': gates, 'loss_report': loss_report, 'provenance': provenance}
