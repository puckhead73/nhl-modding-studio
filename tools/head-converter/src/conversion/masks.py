"""Donor anatomy masks derived from the destination fixture itself (numpy only).

Every mask is computed from the donor exchange (positions, triangles, UVs, connected components) under an explicit
atlas-region POLICY, never from vertex ids of another donor. The default policy is the measured NHL atlas family
'nhl-head-atlas-a' shared exactly by NHL19 Bouwmeester/Gaborik and NHL21 Ekblad (3157/3157 UV twins, coverage IoU
1.0; local/runs/claude/phase3-nhl21-inspect/uv_template_compare.json). Rectangles are in the native atlas UV
(top-down V). A profile that does not share this template must supply its own policy or the fit refuses.
"""
import numpy as np
from . import geometry as G, face_quality as FQ

ATLAS_POLICY_NHL_HEAD_A = {
    'id': 'nhl-head-atlas-a', 'provenance': 'measured on NHL19 Bouwmeester/Gaborik and NHL21 Ekblad fixtures (Studio calibration rectangles + phase-3 template comparison)',
    'oral_rect': {'v_min': 0.84, 'u_outside': [0.405, 0.595]},          # oral islands: v > 0.84 and (u < 0.405 or u > 0.595)
    'lash_band': {'v': [0.3273, 0.3286], 'u': [[0.367, 0.414], [0.585, 0.633]]},
    'ear_core': {'u': [[0.16, 0.26], [0.74, 0.84]], 'v': [0.31, 0.48], 'min_abs_x_m': 0.065, 'rings': 2},
    'mouth_band': {'u': [0.43, 0.57], 'v': [0.51, 0.52]},
    'face_window': {'u': [0.12, 0.88], 'v': [0.10, 0.60], 'feather_texels': 24},
    'attachment': {'bottom_loop_fraction': 0.25, 'rings': 2},
}


def in_rect(U, u, v):
    return (U[:, 0] > u[0]) & (U[:, 0] < u[1]) & (U[:, 1] > v[0]) & (U[:, 1] < v[1])


def derive(P, T, U, components, policy=ATLAS_POLICY_NHL_HEAD_A):
    """P: donor head positions (Blender frame), T triangles, U native UV (top-down), components: list of vertex-id lists.
    Returns dict of boolean masks / id arrays plus the seam groups and adjacency used everywhere downstream."""
    n = len(P); dup = G.seam_groups(P); adj = G.adjacency(n, T, dup)
    outer = FQ.outer_shell_faces(P, T); on_skin = np.zeros(n, bool); on_skin[np.unique(T[outer])] = True
    comps = [np.array(c, int) for c in components]; main = max(comps, key=len)
    o = policy['oral_rect']; oral_rect = (U[:, 1] > o['v_min']) & ((U[:, 0] < o['u_outside'][0]) | (U[:, 0] > o['u_outside'][1]))
    oral_islands = [c for c in comps if len(c) < len(main) and (~on_skin[c]).all() and oral_rect[c].all()]
    oral_ids = np.concatenate(oral_islands) if oral_islands else np.zeros(0, int); is_oral = np.zeros(n, bool); is_oral[oral_ids] = True
    interior_main = ~on_skin & ~is_oral
    a = policy['attachment']; bl = G.bottom_loop(P, T, a['bottom_loop_fraction']); frozen = G.grow(bl, adj, a['rings']); is_frozen = np.zeros(n, bool); is_frozen[frozen] = True
    lb = policy['lash_band']; lash = ((U[:, 1] > lb['v'][0]) & (U[:, 1] < lb['v'][1])) & np.any([(U[:, 0] > u0) & (U[:, 0] < u1) for u0, u1 in lb['u']], axis=0)
    ec = policy['ear_core']; ear_core = np.any([(U[:, 0] > u0) & (U[:, 0] < u1) for u0, u1 in ec['u']], axis=0) & (U[:, 1] > ec['v'][0]) & (U[:, 1] < ec['v'][1]) & (np.abs(P[:, 0]) > ec['min_abs_x_m'])
    ear_ids = G.grow(np.flatnonzero(ear_core), adj, ec['rings'])
    mb = policy['mouth_band']; mouth_band = in_rect(U, mb['u'], mb['v'])
    return {'policy_id': policy['id'], 'dup': dup, 'adj': adj, 'outer_faces': outer, 'on_skin': on_skin, 'oral_islands': oral_islands, 'oral_ids': oral_ids, 'is_oral': is_oral,
            'interior_main': interior_main, 'bottom_loop': bl, 'frozen': frozen, 'is_frozen': is_frozen, 'lash': lash, 'ear_core': ear_core, 'ear_ids': ear_ids, 'mouth_band': mouth_band,
            'oral_rect_on_skin_count': int((oral_rect & on_skin).sum())}


def provenance(masks, policy=ATLAS_POLICY_NHL_HEAD_A, miss_px=None, rear_y=None, rear_blend_rings=None, extra=None):
    """JSON-able mask provenance (rules + ids), mirroring the accepted masks.json layout."""
    m = masks
    out = {'provenance': f"all masks derived from the donor fixture under atlas policy {policy['id']}; no other donor's vertex ids used",
           'attachment_loop': {'rule': f"open-boundary vertices in the lowest {int(policy['attachment']['bottom_loop_fraction']*100)}% of the open set z range, + {policy['attachment']['rings']} graph rings (seam groups merged)", 'bottom_loop': m['bottom_loop'].tolist(), 'frozen_ids': m['frozen'].tolist()},
           'oral_islands': {'rule': f"head connected components other than the main one, fully occluded (no outer-shell face) and fully inside the atlas oral rectangle v>{policy['oral_rect']['v_min']} & (u<{policy['oral_rect']['u_outside'][0]}|u>{policy['oral_rect']['u_outside'][1]})", 'components': [c.tolist() for c in m['oral_islands']], 'ids': m['oral_ids'].tolist(), 'oral_rect_vertices_on_skin_not_treated_as_oral': m['oral_rect_on_skin_count']},
           'mouth_interior_main_component': {'rule': 'occluded (outer-shell test on the donor) and not an oral island', 'ids': np.flatnonzero(m['interior_main']).tolist()},
           'lash_strips': {'rule': f"atlas band {policy['lash_band']}", 'ids': np.flatnonzero(m['lash']).tolist()},
           'ear_core': {'rule': f"atlas bands {policy['ear_core']} (+rings in the neck stage)", 'ids': np.flatnonzero(m['ear_core']).tolist()},
           'seam_groups': m['dup']}
    if extra: out.update(extra)
    return out
