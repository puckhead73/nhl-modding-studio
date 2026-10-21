"""Atlas rasterisation and coverage measurement (port of experiments/claude/measure/atlas_coverage.py)."""
import numpy as np


def rasterize(uv, tris, size, with_bary=False):
    """Texel -> triangle index (-1 = unused), UVs given V-down in [0,1]. Optionally barycentrics."""
    uv = np.asarray(uv, float); tris = np.asarray(tris)
    tri_id = np.full((size, size), -1, np.int32); bary = np.zeros((size, size, 3), np.float32) if with_bary else None
    uvp = uv * size
    for ti, t in enumerate(tris):
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
        inside = (l1 >= -1e-6) & (l2 >= -1e-6) & (l3 >= -1e-6)
        sub = tri_id[y0:y1 + 1, x0:x1 + 1]; take = inside & (sub == -1); sub[take] = ti
        if with_bary: bary[y0:y1 + 1, x0:x1 + 1][take] = np.stack([l1, l2, l3], -1)[take]
    return (tri_id, bary) if with_bary else tri_id


def tps_warp(anchors, coef, p):
    anchors = np.asarray(anchors, float); coef = np.asarray(coef, float); p = np.asarray(p, float); n = len(anchors)
    r2 = ((p[:, None, :] - anchors[None, :, :]) ** 2).sum(2)
    return (r2 * np.log(np.maximum(r2, 1e-20))) @ coef[:n] + np.c_[np.ones(len(p)), p] @ coef[n:]


def coverage_holes(dest_uv, dest_tris, src_uv_vup, src_tris, warp, size):
    """Destination texels used by dest_tris whose warped UV lies outside the source chart. Returns (used, uncovered)."""
    dest = rasterize(dest_uv, dest_tris, size); used = dest >= 0
    yy, xx = np.mgrid[:size, :size]; q = np.c_[(xx[used] + .5) / size, (yy[used] + .5) / size]
    mapped = warp(q); cov = rasterize(np.asarray(src_uv_vup, float), src_tris, size * 2) >= 0
    cx = np.clip((mapped[:, 0] * size * 2).astype(int), 0, size * 2 - 1); cy = np.clip(((1 - mapped[:, 1]) * size * 2).astype(int), 0, size * 2 - 1)
    uncovered = np.zeros((size, size), bool); uncovered[yy[used], xx[used]] = ~cov[cy, cx]
    return used, uncovered
