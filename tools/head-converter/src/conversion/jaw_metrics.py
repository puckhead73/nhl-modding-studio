"""Tessellation-independent jaw/neck metrics (port of experiments/claude/measure/jaw_analysis.py)."""
import numpy as np
from . import geometry as G


def slice_mesh(P, T, axis, v0):
    P = np.asarray(P, float); T = np.asarray(T); a, b, c = P[T[:, 0]], P[T[:, 1]], P[T[:, 2]]; pts = []
    for (p, q) in ((a, b), (b, c), (c, a)):
        dp = p[:, axis] - v0; dq = q[:, axis] - v0; cross = (dp * dq) < 0
        t = dp / (dp - dq + 1e-30); pts.append(np.where(cross[:, None], p + (q - p) * t[:, None], np.nan))
    pts = np.stack(pts, axis=1); ok = ~np.isnan(pts[:, :, 0]); two = ok.sum(1) == 2
    return np.array([pts[i][ok[i]] for i in np.flatnonzero(two)]).reshape(-1, 2, 3)


def chain(seg):
    if len(seg) == 0: return []
    used = np.zeros(len(seg), bool); chains = []; ends = seg.reshape(-1, 3)
    for start in range(len(seg)):
        if used[start]: continue
        used[start] = True; line = [seg[start, 0], seg[start, 1]]
        for direction in (1, 0):
            while True:
                tip = line[-1] if direction else line[0]
                d = np.linalg.norm(ends - tip, axis=1); d[used.repeat(2)] = np.inf; j = int(np.argmin(d))
                if d[j] > 1e-6: break
                si, ei = j // 2, j % 2; used[si] = True; other = seg[si, 1 - ei]
                line.append(other) if direction else line.insert(0, other)
        chains.append(np.array(line))
    return chains


def turning(line, step=0.004, window=0.01):
    """Resample a polyline by arc length and return (points, turning angle in degrees over `window`)."""
    d = np.r_[0, np.cumsum(np.linalg.norm(np.diff(line, axis=0), axis=1))]
    if d[-1] < window * 3: return None, None
    s = np.arange(0, d[-1], step); r = np.c_[np.interp(s, d, line[:, 0]), np.interp(s, d, line[:, 1]), np.interp(s, d, line[:, 2])]
    k = max(1, int(round(window / step))); v1 = r[k:-k] - r[:-2 * k]; v2 = r[2 * k:] - r[k:-k]
    cosang = (v1 * v2).sum(1) / (np.linalg.norm(v1, axis=1) * np.linalg.norm(v2, axis=1) + 1e-30)
    return r[k:-k], np.degrees(np.arccos(np.clip(cosang, -1, 1)))


def max_turning(P, T, axis, v0, keep_fn, window_fn):
    seg = slice_mesh(P, T, axis, v0)
    if len(seg) == 0: return None
    mid = seg.mean(1); seg = seg[keep_fn(mid)]; ch = chain(seg)
    if not ch: return None
    ch.sort(key=lambda c: -np.linalg.norm(np.diff(c, axis=0), axis=1).sum()); r, ang = turning(ch[0])
    if r is None: return None
    win = window_fn(r)
    if not win.any(): return None
    i = int(np.argmax(np.where(win, ang, -1))); return {'max_turn_deg': float(ang[i]), 'at': r[i].tolist()}


def region_distance(P, ids, soup):
    """Distance of P[ids] to a TriangleSoup surface, metres."""
    return np.array([soup.nearest(P[i])[1] for i in ids])
