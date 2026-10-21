"""Surface-based diffuse bake (numpy core; a caller supplies the nearest-surface query and image I/O).

Numeric port of the accepted bake mode G in experiments/claude/texture/surface_bake.py (Blender driver:
experiments/claude/worker/blender_bake.py). Modes A (TPS), B (surface nearest), H (hybrid in the calibrated face
window), F (native fallback with global tone transfer) and G (native fallback with a mesh-solved colour offset that is
zero on the attachment loop + 2 rings and 85 % contrast-compressed towards the CC5 skin tone outside the 4 rings
above the loop) are produced side by side; G is the candidate atlas. Native oral islands are copied from the native
diffuse; unused texels are dilated (32 px). Images are RGBA float32 arrays, row 0 = top (V-down).
"""
import time
import numpy as np
from . import geometry as G

DEFAULTS = {'size': 1024, 'face_window': {'u': [0.12, 0.88], 'v': [0.10, 0.60], 'feather_texels': 24}, 'fallback_mm': {'start': 6.0, 'end': 12.0}, 'G': {'clip': 0.12, 'compress': 0.85, 'loop_rings': 4, 'residual_smooth_passes': 8},
            'oral_rect': {'v_min': 0.84, 'u_outside': [0.405, 0.595]}, 'cc5_bottom_cut_v_topdown': 0.80, 'dilate_px': 32}


def sample(img, q_topdown):
    xy = np.clip(q_topdown * np.array([img.shape[1], img.shape[0]]) - .5, 0, np.array([img.shape[1] - 1.001, img.shape[0] - 1.001]))
    x = xy[:, 0].astype(int); y = xy[:, 1].astype(int); f = xy - np.c_[x, y]
    return (img[y, x] * (1 - f[:, 0, None]) + img[y, x + 1] * f[:, 0, None]) * (1 - f[:, 1, None]) + (img[y + 1, x] * (1 - f[:, 0, None]) + img[y + 1, x + 1] * f[:, 0, None]) * f[:, 1, None]


def rasterize(duv, dtris, size, eps=1e-6):
    tri_id = np.full((size, size), -1, np.int32); bary = np.zeros((size, size, 3), np.float32); uvp = duv * size
    for ti, t in enumerate(dtris):
        a, b, c = uvp[t]
        x0, x1 = int(max(0, np.floor(min(a[0], b[0], c[0])))), int(min(size - 1, np.ceil(max(a[0], b[0], c[0]))))
        y0, y1 = int(max(0, np.floor(min(a[1], b[1], c[1])))), int(min(size - 1, np.ceil(max(a[1], b[1], c[1]))))
        if x1 < x0 or y1 < y0: continue
        yy, xx = np.mgrid[y0:y1 + 1, x0:x1 + 1]; px = xx + .5; py = yy + .5
        det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(det) < 1e-12: continue
        l1 = ((b[0] - px) * (c[1] - py) - (c[0] - px) * (b[1] - py)) / det
        l2 = ((c[0] - px) * (a[1] - py) - (a[0] - px) * (c[1] - py)) / det
        l3 = 1 - l1 - l2
        inside = (l1 >= -eps) & (l2 >= -eps) & (l3 >= -eps)
        sub = tri_id[y0:y1 + 1, x0:x1 + 1]; take = inside & (sub == -1)
        sub[take] = ti; bary[y0:y1 + 1, x0:x1 + 1][take] = np.stack([l1, l2, l3], -1)[take]
    return tri_id, bary


def raster_cover(uv_vup, tris, res):
    img = np.zeros((res, res), bool); uvp2 = uv_vup * res
    for t in tris:
        a, b, c = uvp2[t]
        x0, x1 = int(max(0, np.floor(min(a[0], b[0], c[0])))), int(min(res - 1, np.ceil(max(a[0], b[0], c[0]))))
        y0, y1 = int(max(0, np.floor(min(a[1], b[1], c[1])))), int(min(res - 1, np.ceil(max(a[1], b[1], c[1]))))
        if x1 < x0 or y1 < y0: continue
        yy2, xx2 = np.mgrid[y0:y1 + 1, x0:x1 + 1]; px = xx2 + .5; py = yy2 + .5
        det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(det) < 1e-12: continue
        l1 = ((b[0] - px) * (c[1] - py) - (c[0] - px) * (b[1] - py)) / det; l2 = ((c[0] - px) * (a[1] - py) - (a[0] - px) * (c[1] - py)) / det; l3 = 1 - l1 - l2
        img[y0:y1 + 1, x0:x1 + 1] |= (l1 >= -1e-6) & (l2 >= -1e-6) & (l3 >= -1e-6)
    return img


def bake(dpos, dorig, dtris, duv, spos, stris, suv, src_img, nat_img, warp, nearest, size=1024, policy=DEFAULTS, log=print):
    """dpos fitted head (Blender), dorig donor head, duv native UV (V-down), spos CC5 (already shifted), suv CC5 UV (V-up),
    warp(uv_topdown)->cc5 uv topdown (TPS), nearest(p)->(hit_xyz, tri_index, distance) on the CC5 surface.
    Returns {'atlases': {'A','B','H','F','G'}, 'diag': {...}, 'report': {...}}."""
    t0 = time.time(); fw_ = policy['face_window']; fb = policy['fallback_mm']; Gp = policy['G']; o = policy['oral_rect']
    tri_id, bary = rasterize(duv, dtris, size); used = tri_id >= 0
    yy, xx = np.mgrid[:size, :size]; uy, ux = yy[used], xx[used]; tid = tri_id[used]; bb = bary[used]
    q = np.c_[(ux + .5) / size, (uy + .5) / size]
    log(f'raster {int(used.sum())} texels {round(time.time() - t0, 1)} s')
    mapped = warp(q); colA = sample(src_img, mapped)
    cov = raster_cover(suv, stris, size * 2)
    cx = np.clip((mapped[:, 0] * size * 2).astype(int), 0, size * 2 - 1); cy = np.clip(((1 - mapped[:, 1]) * size * 2).astype(int), 0, size * 2 - 1)
    holeA = ~cov[cy, cx]
    t1 = time.time()
    P3 = (dpos[dtris[tid, 0]] * bb[:, 0:1] + dpos[dtris[tid, 1]] * bb[:, 1:2] + dpos[dtris[tid, 2]] * bb[:, 2:3])
    sA, sB, sC = spos[stris[:, 0]], spos[stris[:, 1]], spos[stris[:, 2]]
    hits = np.zeros((len(P3), 3)); hti = np.zeros(len(P3), int); dist = np.zeros(len(P3))
    for i, p in enumerate(P3):
        loc, ti, d = nearest(p); hits[i] = loc; hti[i] = ti; dist[i] = d
    a, b, c = sA[hti], sB[hti], sC[hti]
    v0 = b - a; v1 = c - a; v2 = hits - a
    d00 = (v0 * v0).sum(1); d01 = (v0 * v1).sum(1); d11 = (v1 * v1).sum(1); d20 = (v2 * v0).sum(1); d21 = (v2 * v1).sum(1)
    den = d00 * d11 - d01 * d01 + 1e-30; bv = (d11 * d20 - d01 * d21) / den; bw = (d00 * d21 - d01 * d20) / den; bu = 1 - bv - bw
    cuv = suv[stris[hti, 0]] * bu[:, None] + suv[stris[hti, 1]] * bv[:, None] + suv[stris[hti, 2]] * bw[:, None]
    colB = sample(src_img, np.c_[cuv[:, 0], 1 - cuv[:, 1]])
    edge_hit = (1 - cuv[:, 1]) > policy['cc5_bottom_cut_v_topdown']
    dist = np.where(edge_hit, np.maximum(dist, 0.02), dist)
    log(f'surface lookup {round(time.time() - t1, 1)} s; nearest distance mm median {np.median(dist) * 1000} p95 {np.percentile(dist, 95) * 1000} max {dist.max() * 1000}')
    face = (q[:, 0] > fw_['u'][0]) & (q[:, 0] < fw_['u'][1]) & (q[:, 1] > fw_['v'][0]) & (q[:, 1] < fw_['v'][1])
    dx = np.minimum(q[:, 0] - fw_['u'][0], fw_['u'][1] - q[:, 0]) * size; dy = np.minimum(q[:, 1] - fw_['v'][0], fw_['v'][1] - q[:, 1]) * size
    fw = np.clip(np.minimum(dx, dy) / float(fw_['feather_texels']), 0, 1); fw[~face] = 0
    fw = fw * fw * (3 - 2 * fw); fw[holeA] = 0
    colH = colA * fw[:, None] + colB * (1 - fw[:, None])

    def compose(col):
        atlas = np.zeros((size, size, 4), np.float32); atlas[uy, ux] = col
        yy2, xx2 = np.mgrid[:size, :size]; oral = (yy2 / size > o['v_min']) & ((xx2 / size < o['u_outside'][0]) | (xx2 / size > o['u_outside'][1]))
        natr = sample(nat_img, np.c_[(xx2[oral] + .5) / size, (yy2[oral] + .5) / size]); atlas[oral] = natr
        filled = used | oral
        for _ in range(policy['dilate_px']):
            if filled.all(): break
            acc = np.zeros_like(atlas); cnt = np.zeros((size, size), np.float32)
            for dy_ in (-1, 0, 1):
                for dx_ in (-1, 0, 1):
                    if dy_ == 0 and dx_ == 0: continue
                    sh = np.roll(np.roll(atlas, dy_, 0), dx_, 1); shf = np.roll(np.roll(filled, dy_, 0), dx_, 1)
                    acc += sh * shf[..., None]; cnt += shf
            newly = (~filled) & (cnt > 0)
            atlas[newly] = acc[newly] / cnt[newly][:, None]; filled |= newly
        atlas[:, :, 3] = 1
        return atlas
    colN = sample(nat_img, q)
    fN = np.clip((dist - fb['start'] / 1000) / ((fb['end'] - fb['start']) / 1000), 0, 1); fN = fN * fN * (3 - 2 * fN)
    band = (fN > .05) & (fN < .95)
    if band.sum() > 100:
        mu_h = colH[band, :3].mean(0); sd_h = colH[band, :3].std(0) + 1e-6; mu_n = colN[band, :3].mean(0); sd_n = colN[band, :3].std(0) + 1e-6
        colN = colN.copy(); colN[:, :3] = np.clip((colN[:, :3] - mu_n) / sd_n * sd_h + mu_h, 0, 1)
    colF = colH * (1 - fN[:, None]) + colN * fN[:, None]
    atlA = compose(colA); atlB = compose(colB); atlH = compose(colH); atlF = compose(colF)
    t2 = time.time()
    colN_raw = sample(nat_img, q)
    vdist = np.array([nearest(p)[2] for p in dpos])
    dup = G.seam_groups(dorig); adj = G.adjacency(len(dpos), dtris, dup)
    loop = G.bottom_loop(dorig, dtris); frozen = G.grow(loop, adj, 2)
    vH = sample(atlH, duv)[:, :3]; vN = sample(nat_img, duv)[:, :3]
    corr = vH - vN
    band_v = (vdist >= fb['start'] / 1000 - 0.002) & (vdist <= fb['end'] / 1000)
    clip = Gp['clip']
    base = np.median(corr[band_v], axis=0) if band_v.any() else np.zeros(3)
    resid = np.clip(corr - base, -clip, clip)
    for _ in range(Gp['residual_smooth_passes']):
        c2 = resid.copy()
        for i in np.flatnonzero(band_v):
            nn = [j for j in adj[i] if band_v[j]]
            if nn: c2[i] = 0.5 * resid[i] + 0.5 * resid[nn].mean(0)
        resid = c2
    corr = base + resid
    known = ~(vdist > fb['end'] / 1000); corr[frozen] = 0; known[frozen] = True
    free = np.flatnonzero(~known); nb = [np.array(sorted(adj[i])) for i in range(len(dpos))]
    its = G.laplace_fill(corr, free, nb)
    for ids in dup: corr[ids] = corr[ids].mean(0)
    loop_rings = Gp['loop_rings']
    wgt = np.clip(G.ring_distance(frozen, adj, len(dpos), loop_rings) / loop_rings, 0, 1); wgt = wgt * wgt * (3 - 2 * wgt)
    for ids in dup: wgt[ids] = wgt[ids].mean()
    compress = Gp['compress']
    tcorr = corr[dtris[tid, 0]] * bb[:, 0:1] + corr[dtris[tid, 1]] * bb[:, 1:2] + corr[dtris[tid, 2]] * bb[:, 2:3]
    tw = wgt[dtris[tid, 0]] * bb[:, 0] + wgt[dtris[tid, 1]] * bb[:, 1] + wgt[dtris[tid, 2]] * bb[:, 2]
    target = np.median(colH[band, :3], axis=0) if band.any() else colH[:, :3].mean(0)
    cN = colN_raw[:, :3] + tcorr
    cN = target + (cN - target) * (1 - compress * tw[:, None])
    colN_G = colN_raw.copy(); colN_G[:, :3] = np.clip(cN, 0, 1)
    colG = colH * (1 - fN[:, None]) + colN_G * fN[:, None]
    atlG = compose(colG)
    fb_v = vdist > fb['end'] / 1000; feather_v = band_v
    g_stats = {'base_offset_rgb': base.tolist(), 'residual_clip': clip, 'contrast_compression': compress, 'native_exact_rings_above_loop': loop_rings, 'target_skin_rgb': target.tolist(), 'fallback_vertices': int(fb_v.sum()), 'feather_vertices': int(feather_v.sum()), 'frozen_loop_vertices_zero_correction': int(len(frozen)), 'laplace_iterations': its,
               'fallback_boundary_height_m': {'feather_z_min': float(dpos[feather_v, 2].min()) if feather_v.any() else None, 'feather_z_median': float(np.median(dpos[feather_v, 2])) if feather_v.any() else None, 'feather_z_max': float(dpos[feather_v, 2].max()) if feather_v.any() else None,
                                              'fallback_z_max': float(dpos[fb_v, 2].max()) if fb_v.any() else None, 'attachment_loop_z_max': float(dorig[loop, 2].max()), 'attachment_loop_z_min': float(dorig[loop, 2].min()), 'cc5_source_z_min': float(spos[:, 2].min())},
               'correction_rgb_abs_mean_on_feather': float(np.abs(corr[feather_v]).mean()) if feather_v.any() else None, 'correction_rgb_abs_max': float(np.abs(corr).max()),
               'F_global_transfer_vs_G_rgb_rmse_in_fallback': float(np.sqrt(((colF[fN > .999, :3] - colG[fN > .999, :3]) ** 2).mean())) if (fN > .999).any() else None, 'seconds': round(time.time() - t2, 1)}

    def boundary_step(col):
        inner = (fN > .02) & (fN < .2); outer = (fN > .8) & (fN < .98)
        return float(np.abs(col[inner, :3].mean(0) - col[outer, :3].mean(0)).sum())
    g_stats['feather_rgb_step_F'] = boundary_step(colF); g_stats['feather_rgb_step_G'] = boundary_step(colG)
    g_stats['fallback_vs_native_rgb_mean_abs_F'] = float(np.abs(colF[fN > .999, :3] - colN_raw[fN > .999, :3]).mean()) if (fN > .999).any() else None
    g_stats['fallback_vs_native_rgb_mean_abs_G'] = float(np.abs(colG[fN > .999, :3] - colN_raw[fN > .999, :3]).mean()) if (fN > .999).any() else None
    # diagnostics
    cm = np.zeros((size, size, 4), np.float32); mag = np.clip(np.abs(tcorr).sum(1) / 0.6, 0, 1); cm[uy, ux] = np.c_[mag, 1 - mag, fN, np.ones_like(mag)]
    hole_img = np.zeros((size, size, 4), np.float32); hole_img[uy, ux] = np.where(holeA[:, None], [1, 0, 0, 1], [.4, .4, .4, 1])
    dm = np.zeros((size, size, 4), np.float32); dn = np.clip(dist / 0.02, 0, 1); dm[uy, ux] = np.c_[dn, 1 - dn, np.zeros_like(dn), np.ones_like(dn)]
    # region coverage of holes / fallback in atlas regions (for the bake receipt)
    regions = {'face_window': face, 'left_edge_rear': q[:, 0] < 0.12, 'right_edge_rear': q[:, 0] > 0.88, 'chest_bottom': q[:, 1] > 0.60}
    region_cov = {k: {'texels': int(m.sum()), 'A_holes': int((holeA & m).sum()), 'native_fallback': int(((fN > .999) & m).sum()), 'feathered': int((((fN > .001) & (fN < .999)) & m).sum())} for k, m in regions.items()}
    report = {'size': size, 'used_texels': int(used.sum()), 'A_hole_texels': int(holeA.sum()), 'B_cc5_bottom_edge_hits': int(edge_hit.sum()), 'B_nearest_distance_mm': {'median': float(np.median(dist) * 1000), 'p95': float(np.percentile(dist, 95) * 1000), 'max': float(dist.max() * 1000), 'over_5mm': int((dist > .005).sum()), 'over_10mm': int((dist > .01).sum())},
              'H_texels_from_A': int((fw > .999).sum()), 'H_texels_from_B': int((fw < .001).sum()), 'H_feathered': int(((fw > .001) & (fw < .999)).sum()), 'F_texels_native_fallback': int((fN > .999).sum()), 'F_feathered': int(((fN > .001) & (fN < .999)).sum()),
              'G': g_stats, 'A_vs_B_rgb_rmse_where_A_covered': float(np.sqrt(((colA[~holeA, :3] - colB[~holeA, :3]) ** 2).mean())), 'A_vs_B_rgb_rmse_face_window': float(np.sqrt(((colA[face & ~holeA, :3] - colB[face & ~holeA, :3]) ** 2).mean())),
              'region_coverage': region_cov, 'policy': policy, 'colour_space': 'PNG bytes read as Non-Color (no gamma transform), written back the same way', 'mips': 'none here (destination policy: native BC1 + 11 mips by the packer)', 'seconds': round(time.time() - t0, 1)}
    return {'atlases': {'A': atlA, 'B': atlB, 'H': atlH, 'F': atlF, 'G': atlG}, 'diag': {'G_correction_magnitude': cm, 'A_holes': hole_img, 'B_nearest_distance_0-20mm': dm}, 'report': report}
