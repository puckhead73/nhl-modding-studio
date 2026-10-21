"""Combined-eye component placement onto the source eye spheres (numpy only).

Port of experiments/claude/nhl21/eyes_place_nhl21.py: each connected component of the donor eye submesh is
sphere-fitted and moved by a per-component uniform scale + translation onto the CC5 eye sphere of the same side
(orientation kept, so native normals/tangents/UVs stay valid; the native packer verifies the similarity). A qualified
profile must allow this operation (NHL21 Ekblad combined eyes: accepted by user runtime test on two fixtures; NHL19 caps
use a different, cap-specific policy). Pivot provenance: CC5 sphere centre; the native rig eye pivot is unknown.
"""
import numpy as np
from .eye_fit import sphere_fit, place_cap


def place_eyes(eye_positions_donor, components, cc5_eye_positions_shifted, cc5_eye_material_ids):
    """eye_positions_donor: Blender frame (n,3); components: list of vertex-id lists; cc5_eye_positions_shifted:
    CC5 eye records + source_shift; cc5_eye_material_ids: {'Std_Eye_R': ids, ...}. Returns (positions, report)."""
    Pe = np.asarray(eye_positions_donor, float); Q = np.asarray(cc5_eye_positions_shifted, float); Pn = Pe.copy()
    cc5 = {}
    for name in ('Std_Eye_R', 'Std_Cornea_R', 'Std_Eye_L', 'Std_Cornea_L'):
        ids = np.asarray(cc5_eye_material_ids[name]); c, r, res = sphere_fit(Q[ids]); cc5[name] = (c, r, float(np.sqrt((res ** 2).mean()) * 1000))
    report = {'method': 'per-component uniform scale + translation onto the CC5 Std_Eye sphere of the same side (sphere centre pivot; native pivot unknown)',
              'cc5_spheres': {k: {'centre': v[0].tolist(), 'radius_mm': v[1] * 1000, 'rms_mm': v[2]} for k, v in cc5.items()}, 'components': []}
    for c in components:
        ids = np.asarray(c, int); co, ro, _ = sphere_fit(Pe[ids]); side = 'L' if co[0] > 0 else 'R'; cc, rc, _ = cc5['Std_Eye_' + side]
        placed, info = place_cap(Pe[ids], cc, rc); Pn[ids] = placed
        report['components'].append({'side': side, 'vertices': int(len(ids)), 'donor_centre_m': co.tolist(), 'donor_radius_mm': ro * 1000, 'target_centre_m': cc.tolist(), 'target_radius_mm': rc * 1000, 'uniform_scale': info['scale'], 'translation_mm': ((cc - co) * 1000).tolist()})
    return Pn, report


def verify_similarity(eye_positions_donor, eye_positions_new, components):
    """Each component must be donor component under a positive uniform scale + translation (rms < 1e-6 mm)."""
    Pe = np.asarray(eye_positions_donor, float); Pn = np.asarray(eye_positions_new, float); out = {}; ok = True
    for ci, c in enumerate(components):
        ids = np.asarray(c, int); co, ro, _ = sphere_fit(Pe[ids]); cn, rn, _ = sphere_fit(Pn[ids]); sc = rn / ro; recon = (Pe[ids] - co) * sc + cn
        rms = float(np.sqrt(((recon - Pn[ids]) ** 2).sum(1).mean()) * 1000)
        out[ci] = {'side': 'L' if co[0] > 0 else 'R', 'uniform_scale': float(sc), 'translation_m': (cn - co).tolist(), 'reconstruction_rms_mm': rms, 'sphere_centre_m': cn.tolist(), 'sphere_radius_mm': float(rn * 1000), 'donor_sphere_radius_mm': float(ro * 1000)}
        ok &= rms < 1e-6 and sc > 0
    return ok, out
