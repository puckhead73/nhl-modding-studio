"""Destination-topology UV-registration fit of a CC5 source onto an NHL-atlas donor head (numpy only).

Numerical port of experiments/claude/nhl21/uv_fit_nhl21.py (the accepted NHL21 Turgeon/Fedorov recipe, mouth_rule v1);
operation order is kept so results are bit-identical to the accepted runs (checked by the parity harness).

Per native head vertex: native UV (top-down) -> TPS (family calibration: 45 native-atlas -> CC5-atlas pixel pairs)
-> CC5 UV -> CC5 triangle containing that UV (2048^2 raster, exact barycentrics) -> 3D point on the CC5 surface
+ rigid shift (y from eye-depth means, z from crown alignment). Vertices whose warped UV misses the CC5 chart by more
than `miss_px` and the whole posterior head (donor y > rear_y behind the ears; ear sets excluded) get a radial
correspondence above band_top (ray from a head centre through the donor vertex to the CC5 surface), blended into
the TPS fit over `rear_blend_rings` graph rings; anything without a hit and the neck band are harmonic in the
displacement field. Oral islands follow the mean mouth-band displacement; equal-UV lip layers keep their native
offset from the frontmost record; seam duplicates stay coincident; lash strips follow their nearest skin triangle;
the attachment loop + rings is frozen. Eyes / cards / haircap are never touched here.
"""
import time
import numpy as np
from . import geometry as G, face_quality as FQ, masks as M

DEFAULTS = {'band_top': 1.775, 'fit_top': 1.79, 'miss_px': 1.5, 'raster': 2048, 'rear_y': 0.005, 'rear_blend_rings': 4}


def tps_warp(anchors, coef, p):
    anchors = np.asarray(anchors, float); coef = np.asarray(coef, float); p = np.asarray(p, float); nA = len(anchors)
    r2 = ((p[:, None, :] - anchors[None, :, :]) ** 2).sum(2)
    return (r2 * np.log(np.maximum(r2, 1e-20))) @ coef[:nA] + np.c_[np.ones(len(p)), p] @ coef[nA:]


def tps_solve(pairs_pixels, ref_size=1024):
    """Thin-plate-spline coefficients from landmark pairs in pixel space (Studio calibration method, reproduced)."""
    anchors = np.array([a for a, b in pairs_pixels], float) / ref_size; targets = np.array([b for a, b in pairs_pixels], float) / ref_size
    n = len(anchors); r2 = ((anchors[:, None, :] - anchors[None, :, :]) ** 2).sum(2); K = r2 * np.log(np.maximum(r2, 1e-20))
    P = np.c_[np.ones(n), anchors]; A = np.block([[K + np.eye(n) * 1e-9, P], [P.T, np.zeros((3, 3))]])
    return np.linalg.solve(A, np.vstack([targets, np.zeros((3, 2))]))


def uv_raster(SU_td, ST, R):
    """Texel -> CC5 triangle index (top-down UV, R x R)."""
    tri_id = np.full((R, R), -1, np.int32); uvp = SU_td * R
    for ti, t in enumerate(ST):
        a, b, c = uvp[t]
        x0, x1 = int(max(0, np.floor(min(a[0], b[0], c[0])))), int(min(R - 1, np.ceil(max(a[0], b[0], c[0]))))
        y0, y1 = int(max(0, np.floor(min(a[1], b[1], c[1])))), int(min(R - 1, np.ceil(max(a[1], b[1], c[1]))))
        if x1 < x0 or y1 < y0: continue
        yy, xx = np.mgrid[y0:y1 + 1, x0:x1 + 1]; px = xx + .5; py = yy + .5; det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(det) < 1e-14: continue
        l1 = ((b[0] - px) * (c[1] - py) - (c[0] - px) * (b[1] - py)) / det; l2 = ((c[0] - px) * (a[1] - py) - (a[0] - px) * (c[1] - py)) / det; l3 = 1 - l1 - l2
        inside = (l1 >= -1e-4) & (l2 >= -1e-4) & (l3 >= -1e-4); sub = tri_id[y0:y1 + 1, x0:x1 + 1]; sub[inside & (sub == -1)] = ti
    return tri_id


def fit_head(P, T, U, components, S0, ST, SU, source_eye_positions, donor_eye_positions, tps_pairs_pixels, tps_coefficients, *,
             band_top=1.775, fit_top=1.79, miss_px=1.5, raster=2048, rear_y=0.005, rear_blend_rings=4, policy=M.ATLAS_POLICY_NHL_HEAD_A):
    """P donor head (Blender frame), T, U native UV top-down, components (list of vertex-id lists); S0/ST/SU CC5 head
    (SU is V-up); source/donor eye positions (Blender frame) for the depth shift; TPS ledger arrays.
    Returns (fit positions, mode array, ledger dict, report dict, masks dict, masks provenance dict)."""
    t0 = time.time(); n = len(P); R = raster
    SU_td = np.c_[SU[:, 0], 1 - SU[:, 1]]
    mk = M.derive(P, T, U, components, policy); dup = mk['dup']; adj = mk['adj']; outer = mk['outer_faces']; on_skin = mk['on_skin']
    oral_ids = mk['oral_ids']; is_oral = mk['is_oral']; frozen = mk['frozen']; is_frozen = mk['is_frozen']; lash = mk['lash']; ear_core = mk['ear_core']; mouth_band = mk['mouth_band']
    anchors = np.array([a for a, b in tps_pairs_pixels], float) / 1024; coef = np.array(tps_coefficients)
    mapped = tps_warp(anchors, coef, U)
    tri_id = uv_raster(SU_td, ST, R)

    def bary2d(q, ti):
        a, b, c = SU_td[ST[ti, 0]], SU_td[ST[ti, 1]], SU_td[ST[ti, 2]]
        det = (b[:, 0] - a[:, 0]) * (c[:, 1] - a[:, 1]) - (c[:, 0] - a[:, 0]) * (b[:, 1] - a[:, 1])
        l1 = ((b[:, 0] - q[:, 0]) * (c[:, 1] - q[:, 1]) - (c[:, 0] - q[:, 0]) * (b[:, 1] - q[:, 1])) / det
        l2 = ((c[:, 0] - q[:, 0]) * (a[:, 1] - q[:, 1]) - (a[:, 0] - q[:, 0]) * (c[:, 1] - q[:, 1])) / det
        return np.c_[l1, l2, 1 - l1 - l2]

    qi = np.clip((mapped * R).astype(int), 0, R - 1); hit_tri = tri_id[qi[:, 1], qi[:, 0]]
    uv_dist = np.zeros(n); soup_uv = G.TriangleSoup(np.c_[SU_td, np.zeros(len(SU_td))], ST)
    for i in np.flatnonzero(hit_tri < 0):
        q, d, k = soup_uv.nearest_face(np.array([mapped[i, 0], mapped[i, 1], 0.0])); hit_tri[i] = k; uv_dist[i] = d; mapped[i] = q[:2]
    B = np.clip(bary2d(mapped, hit_tri), 0, 1); B /= B.sum(1)[:, None]
    sampled0 = (S0[ST[hit_tri, 0]] * B[:, :1] + S0[ST[hit_tri, 1]] * B[:, 1:2] + S0[ST[hit_tri, 2]] * B[:, 2:])
    SE = np.asarray(source_eye_positions, float); DE = np.asarray(donor_eye_positions, float)
    shift = np.array([0.0, DE[:, 1].mean() - SE[:, 1].mean(), P[:, 2].max() - S0[:, 2].max()])
    S = S0 + shift; sampled = sampled0 + shift
    covered = uv_dist <= miss_px / 1024
    D = sampled - P; mode = np.full(n, 'uv', object); below = P[:, 2] < band_top
    ear_set = np.zeros(n, bool); ear_set[G.grow(np.flatnonzero(ear_core), adj, 2)] = True
    rear = (P[:, 1] > rear_y) & ~ear_set & ~is_oral & on_skin
    mode[~covered | rear] = 'uncovered'; mode[below] = 'band'; mode[is_frozen] = 'frozen'; D[is_frozen] = 0
    centre = np.array([0.0, float(S[S[:, 2] > DE[:, 2].mean(), 1].mean()), float(DE[:, 2].mean() + 0.02)])
    soup = G.TriangleSoup(S, ST); radial_hit = 0
    for i in np.flatnonzero((mode == 'uncovered') & ~below):
        d = P[i] - centre; d /= np.linalg.norm(d)
        t = soup.ray_hit(centre, d)
        if t is None: continue
        D[i] = centre + d * t - P[i]; mode[i] = 'radial'; radial_hit += 1
    mouth_delta = D[mouth_band & covered].mean(0); D[is_oral] = mouth_delta; mode[is_oral] = 'oral'
    if rear_blend_rings:
        near_rear = np.zeros(n, bool); near_rear[G.grow(np.flatnonzero(mode == 'radial'), adj, rear_blend_rings)] = True
        mode[np.flatnonzero(near_rear & (mode == 'uv'))] = 'blend'
    free = np.flatnonzero((mode == 'uncovered') | (mode == 'band') | (mode == 'blend'))
    nb = [np.array(sorted(adj[i])) for i in range(n)]
    its = G.laplace_fill(D, free, nb)
    for _ in range(3):
        ids = np.flatnonzero((mode == 'radial') | (mode == 'uncovered') | (mode == 'blend')); Dn = D.copy()
        for i in ids: Dn[i] = 0.5 * D[i] + 0.5 * D[nb[i]].mean(0)
        D = Dn
    fit = P + D
    skinT = T[outer & ~lash[T].any(1) & ~is_oral[T].any(1)]; sk = G.TriangleSoup(P, skinT)
    for i in np.flatnonzero(lash):
        q, d, k = sk.nearest_face(P[i]); t = skinT[k]
        a, b, c = P[t]; v0 = b - a; v1 = c - a; v2 = q - a; d00 = v0 @ v0; d01 = v0 @ v1; d11 = v1 @ v1; d20 = v2 @ v0; d21 = v2 @ v1; den = d00 * d11 - d01 * d01 + 1e-30
        bv = (d11 * d20 - d01 * d21) / den; bw = (d00 * d21 - d01 * d20) / den; bu = 1 - bv - bw
        fit[i] = P[i] + (fit[t[0]] * bu + fit[t[1]] * bv + fit[t[2]] * bw) - q; mode[i] = 'lash'
    groups = {}
    for i, u in enumerate(map(tuple, U)): groups.setdefault(u, []).append(i)
    layer_groups = [g for g in groups.values() if len(g) > 1]
    for g in layer_groups:
        front = min(g, key=lambda i: P[i, 1])
        for i in g:
            if np.linalg.norm(P[i] - P[front]) > 1e-5: fit[i] = fit[front] + P[i] - P[front]
    for ids in dup: fit[ids] = fit[ids].mean(0)
    fit[frozen] = P[frozen]
    if abs(fit[:, 2].max() - P[:, 2].max()) >= 2e-3: raise ValueError('crown alignment lost')
    dist_cov = np.array([soup.nearest(fit[i])[1] for i in np.flatnonzero((mode == 'uv') & ~lash)]) * 1000
    q = G.triangle_quality(P, fit, T); m_fit = FQ.face_metrics(fit, T, dup); m_don = FQ.face_metrics(P, T, dup)
    report = {'params': {'band_top': band_top, 'fit_top': fit_top, 'miss_px': miss_px, 'raster': R, 'rear_y': rear_y, 'rear_blend_rings': rear_blend_rings, 'mouth_rule': 'v1', 'atlas_policy': policy['id']},
              'shift_m': shift.tolist(), 'radial_centre_m': centre.tolist(), 'modes': {k: int((mode == k).sum()) for k in ('uv', 'uncovered', 'radial', 'blend', 'band', 'frozen', 'oral', 'lash')},
              'uv_miss_px': {'max': float(uv_dist.max() * 1024), 'over_miss_px': int((uv_dist > miss_px / 1024).sum())}, 'radial_hits': radial_hit, 'laplace_iterations': its,
              'covered_fit_to_cc5_mm': {'median': float(np.median(dist_cov)), 'p95': float(np.percentile(dist_cov, 95)), 'max': float(dist_cov.max())},
              'mouth_delta_mm': (mouth_delta * 1000).tolist(), 'max_move_vs_donor_mm': float(np.linalg.norm(fit - P, axis=1).max() * 1000), 'crown_z': float(fit[:, 2].max()), 'donor_crown_z': float(P[:, 2].max()),
              'frozen_max_error_m': float(np.abs(fit[frozen] - P[frozen]).max()), 'seam_max_gap_m': max((float(np.linalg.norm(fit[ids] - fit[ids[0]], axis=1).max()) for ids in dup), default=0.0),
              'triangle_quality_vs_donor': q, 'fit_metrics': {'inverted_local': int(m_fit['inverted_local'].sum()), 'folded_edge': int(m_fit['folded_edge'].sum()), 'sliver': int(m_fit['sliver'].sum())},
              'donor_metrics': {'inverted_local': int(m_don['inverted_local'].sum()), 'folded_edge': int(m_don['folded_edge'].sum()), 'sliver': int(m_don['sliver'].sum())}, 'seconds': round(time.time() - t0, 1)}
    ledger = {'pairs_pixels': tps_pairs_pixels, 'coefficients': np.asarray(tps_coefficients).tolist(), 'source_shift': shift.tolist(), 'oral_ids': oral_ids.tolist(),
              'placement': {'axis': 'Blender Z (native Y)', 'donor_scalp_z': float(P[:, 2].max()), 'incoming_scalp_z': float(S0[:, 2].max()), 'translation': shift.tolist(), 'frozen_native_ids': frozen.tolist(), 'frozen_count': int(len(frozen)), 'band_top': band_top, 'fit_top': fit_top}}
    extra = {'mouth_band_for_oral_delta': {'rule': f"{policy['mouth_band']} & covered", 'ids': np.flatnonzero(mouth_band & covered).tolist()},
             'uncovered_by_cc5_chart': {'rule': f'TPS-warped UV misses the CC5 chart by > {miss_px} px', 'ids': np.flatnonzero(~covered).tolist()},
             'posterior_radial': {'rule': f'donor y > {rear_y} (behind the ears), skin, not ear/oral: radial correspondence instead of the TPS, blended over {rear_blend_rings} rings', 'ids': np.flatnonzero(rear).tolist(), 'modes_after': {k: np.flatnonzero(mode == k).tolist() for k in ('radial', 'blend', 'uncovered')}},
             'lip_layer_groups': layer_groups}
    return fit, mode.astype(str), ledger, report, mk, M.provenance(mk, policy, extra=extra)
