"""Geometry measurements for candidate receipts (numpy only): jaw-region distance to source, lid screening, eye
diagnostic, seam displacement, affected region. Ports of experiments/claude/measure/jaw_analysis.py (distance part),
experiments/claude/nhl21/lid_screen_nhl21.py and eye_diagnostic_nhl21.py. All values are rest-pose screening metrics.
"""
import numpy as np
from . import geometry as G, face_quality as FQ
from .eye_fit import sphere_fit

JAW_REGION = {'abs_x': [0.035, 0.085], 'y': [-0.03, 0.06], 'z': [1.68, 1.78]}   # measured on the NHL atlas-A donors (Blender metres)


def jaw_region_distance(orig, fit, T, uv, src, stris, ear_core_mask, region=JAW_REGION):
    soup = G.TriangleSoup(src, stris)
    ear_ids = G.grow(np.flatnonzero(ear_core_mask), G.adjacency(len(orig), T), 2); ear_v = np.zeros(len(orig), bool); ear_v[ear_ids] = True
    jaw = (np.abs(orig[:, 0]) > region['abs_x'][0]) & (np.abs(orig[:, 0]) < region['abs_x'][1]) & (orig[:, 1] > region['y'][0]) & (orig[:, 1] < region['y'][1]) & (orig[:, 2] > region['z'][0]) & (orig[:, 2] < region['z'][1]) & ~ear_v
    out = {}
    for s, name in ((1, 'character_left'), (-1, 'character_right')):
        ids = np.flatnonzero(jaw & (orig[:, 0] * s > 0))
        dfit = np.array([soup.nearest(fit[i])[1] for i in ids]) * 1000; ddon = np.array([soup.nearest(orig[i])[1] for i in ids]) * 1000
        out[name] = {'vertices': int(len(ids)), 'fit_to_source_mm': {'median': round(float(np.median(dfit)), 3), 'p95': round(float(np.percentile(dfit, 95)), 3), 'max': round(float(dfit.max()), 3)}, 'donor_to_source_mm': {'median': round(float(np.median(ddon)), 3), 'max': round(float(ddon.max()), 3)}}
    return out


def source_distance(fit, mask, src, stris):
    """Distance of masked head records to the source surface (mm): median/p95/max."""
    soup = G.TriangleSoup(src, stris); ids = np.flatnonzero(mask)
    d = np.array([soup.nearest(fit[i])[1] for i in ids]) * 1000 if len(ids) else np.zeros(0)
    return {'records': int(len(ids)), 'median_mm': float(np.median(d)) if len(d) else None, 'p95_mm': float(np.percentile(d, 95)) if len(d) else None, 'max_mm': float(d.max()) if len(d) else None}


def affected_region(orig, fit, dup, threshold_m=1e-9):
    """Records moved vs donor, max/median move, bounding box of the moved set and seam gap."""
    d = np.linalg.norm(fit - orig, axis=1); moved = np.flatnonzero(d > threshold_m)
    return {'records_moved': int(len(moved)), 'max_move_mm': float(d.max() * 1000), 'median_move_of_moved_mm': float(np.median(d[moved]) * 1000) if len(moved) else 0.0,
            'moved_bbox_min_m': fit[moved].min(0).tolist() if len(moved) else None, 'moved_bbox_max_m': fit[moved].max(0).tolist() if len(moved) else None,
            'seam_max_gap_m': max((float(np.linalg.norm(fit[ids] - fit[ids[0]], axis=1).max()) for ids in dup), default=0.0)}


def lid_screen(donor, after, T, U, eye_orig, eye_new, components, lash_mask):
    outer = FQ.outer_shell_faces(donor, T); skin = np.zeros(len(donor), bool); skin[np.unique(T[outer])] = True; lash = np.asarray(lash_mask, bool)
    res = {'method': 'signed sphere-distance screening on proximity-selected records (+-2 mm of the native eye surface in the donor, front hemisphere, skin, socket band excluded); partial static evidence only, not eyelid contact / visibility / pivot / blink qualification', 'per_side': {}}
    for c in components:
        c = np.asarray(c, int); c0, r0, _ = sphere_fit(eye_orig[c]); c1, r1, _ = sphere_fit(eye_new[c]); side = 'L' if c0[0] > 0 else 'R'
        rad = np.linalg.norm(donor - c0, axis=1); sel = np.flatnonzero(skin & ~lash & (donor[:, 1] < c0[1]) & (donor[:, 0] * np.sign(c0[0]) > 0) & (np.abs(rad - r0) < 0.002))

        def rel(P, cen, r):
            d = (np.linalg.norm(P[sel] - cen, axis=1) - r) * 1000
            return {'n': int(len(sel)), 'median_mm': round(float(np.median(d)), 2), 'min_mm': round(float(d.min()), 2), 'max_mm': round(float(d.max()), 2), 'inside_count': int((d < 0).sum())}
        res['per_side'][side] = {'record_ids': sel.tolist(), 'native_eye': {'centre_m': c0.round(5).tolist(), 'radius_mm': round(r0 * 1000, 3)}, 'placed_eye': {'centre_m': c1.round(5).tolist(), 'radius_mm': round(r1 * 1000, 3), 'uniform_scale': round(r1 / r0, 6), 'translation_mm': ((c1 - c0) * 1000).round(2).tolist()},
                                 'donor_head_vs_native_eye': rel(donor, c0, r0), 'fitted_head_vs_native_eye': rel(after, c0, r0), 'fitted_head_vs_placed_eye': rel(after, c1, r1),
                                 'selected_records_centroid_shift_mm': ((after[sel].mean(0) - donor[sel].mean(0)) * 1000).round(2).tolist()}
    return res


def eye_diagnostic(eye_orig, components, cc5_eye_shifted, cc5_eye_material_ids):
    out = {}
    for c in components:
        ids = np.asarray(c, int); Qe = eye_orig[ids]; co, ro, res = sphere_fit(Qe); side = 'L' if co[0] > 0 else 'R'
        cc, rc, _ = sphere_fit(cc5_eye_shifted[np.asarray(cc5_eye_material_ids['Std_Eye_' + side])])
        out[side] = {'native_centre_m': co.round(5).tolist(), 'native_radius_mm': round(ro * 1000, 3), 'native_sphere_rms_mm': round(float(np.sqrt((res ** 2).mean())) * 1000, 4), 'native_front_y': round(float(Qe[:, 1].min()), 5),
                     'cc5_centre_m': cc.round(5).tolist(), 'cc5_radius_mm': round(rc * 1000, 3), 'centre_offset_mm': ((cc - co) * 1000).round(2).tolist(), 'centre_offset_norm_mm': round(float(np.linalg.norm(cc - co) * 1000), 2), 'radius_ratio_cc5_over_native': round(rc / ro, 4)}
    return out
