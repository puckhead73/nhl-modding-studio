"""Per-triangle geometric quality checks that do not depend on a prior fit (numpy only).

A sign change of a face normal relative to another mesh state is only *evidence* of a problem. These helpers measure
the actual conditions: degeneracy (area collapse, sliver angle), local folds (face normal against the smoothed vertex
normal of its own surface, and the dihedral angle to each neighbour) and self-intersection (exact triangle-triangle
tests on non-adjacent bounding-box candidates; COPLANAR pairs are not detected). All positions are metres in any
consistent frame.
"""
import numpy as np


def face_normals(P, T, normalize=True):
    P = np.asarray(P, float); T = np.asarray(T)
    c = np.cross(P[T[:, 1]] - P[T[:, 0]], P[T[:, 2]] - P[T[:, 0]])
    if not normalize: return c
    return c / np.maximum(np.linalg.norm(c, axis=1)[:, None], 1e-20)


def face_areas(P, T):
    return 0.5 * np.linalg.norm(face_normals(P, T, normalize=False), axis=1)


def min_angles_deg(P, T):
    P = np.asarray(P, float); T = np.asarray(T); a, b, c = P[T[:, 0]], P[T[:, 1]], P[T[:, 2]]
    out = []
    for (p, q, r) in ((a, b, c), (b, c, a), (c, a, b)):
        u = q - p; v = r - p
        cosang = (u * v).sum(1) / np.maximum(np.linalg.norm(u, axis=1) * np.linalg.norm(v, axis=1), 1e-20)
        out.append(np.degrees(np.arccos(np.clip(cosang, -1, 1))))
    return np.min(np.stack(out, 1), axis=1)


def _merged_adjacency(n, T, merge_groups):
    adj = [set() for _ in range(n)]
    for a, b, c in T: adj[a].update((b, c)); adj[b].update((a, c)); adj[c].update((a, b))
    for ids in merge_groups:
        u = set().union(*(adj[i] for i in ids)) - set(ids)
        for i in ids: adj[i] = set(u)
    return adj


def smoothed_vertex_normals(P, T, merge_groups=(), passes=2):
    """Area-weighted vertex normals, seam groups merged, then Laplacian-smoothed `passes` times over the graph."""
    P = np.asarray(P, float); T = np.asarray(T); n = len(P)
    fn = face_normals(P, T, normalize=False); vn = np.zeros((n, 3))
    for k in range(3): np.add.at(vn, T[:, k], fn)
    for ids in merge_groups: vn[ids] = vn[ids].sum(0)
    vn /= np.maximum(np.linalg.norm(vn, axis=1)[:, None], 1e-20)
    adj = _merged_adjacency(n, T, merge_groups)
    for _ in range(passes):
        nv = vn.copy()
        for i in range(n):
            if adj[i]: nv[i] = vn[i] + vn[list(adj[i])].sum(0)
        vn = nv / np.maximum(np.linalg.norm(nv, axis=1)[:, None], 1e-20)
    return vn


def fold_score(P, T, merge_groups=(), passes=2):
    """Dot of each face normal with the mean smoothed vertex normal of its corners. < 0 means the face is inverted
    relative to its own neighbourhood (a real fold), independent of any reference mesh."""
    vn = smoothed_vertex_normals(P, T, merge_groups, passes); fn = face_normals(P, T)
    m = vn[np.asarray(T)].mean(1); m /= np.maximum(np.linalg.norm(m, axis=1)[:, None], 1e-20)
    return (fn * m).sum(1)


def edge_face_map(T, merge_groups=()):
    T = np.asarray(T); n = int(T.max()) + 1; canon = np.arange(n)
    for ids in merge_groups:
        for i in ids: canon[i] = ids[0]
    Tc = canon[T]; ef = {}
    for fi, (a, b, c) in enumerate(Tc):
        for e in ((a, b), (b, c), (c, a)): ef.setdefault((min(e), max(e)), []).append(fi)
    return ef


def max_dihedral_deg(P, T, merge_groups=(), ef=None):
    """Per face: the largest angle between its normal and any edge-neighbour's normal (180 = folded back)."""
    T = np.asarray(T); fn = face_normals(P, T); ef = ef or edge_face_map(T, merge_groups); out = np.zeros(len(T))
    for e, fs in ef.items():
        if len(fs) < 2: continue
        for i in fs:
            for j in fs:
                if i == j: continue
                ang = np.degrees(np.arccos(np.clip(float(fn[i] @ fn[j]), -1, 1))); out[i] = max(out[i], ang)
    return out


def _tri_tri_intersect(p1, q1, r1, p2, q2, r2, eps=1e-12):
    """Exact triangle-triangle intersection (Möller 1997 interval overlap). Coplanar pairs return False."""
    def signs(a, b, c, p, q, r):
        n = np.cross(b - a, c - a); d = -n @ a
        return np.array([n @ p + d, n @ q + d, n @ r + d]), n
    d2, n1 = signs(p1, q1, r1, p2, q2, r2)
    if (d2 > eps).all() or (d2 < -eps).all(): return False
    d1, n2 = signs(p2, q2, r2, p1, q1, r1)
    if (d1 > eps).all() or (d1 < -eps).all(): return False
    if (np.abs(d1) < eps).all() and (np.abs(d2) < eps).all(): return False
    D = np.cross(n1, n2); ax = int(np.argmax(np.abs(D)))

    def interval(p, q, r, d):
        pp = np.array([p[ax], q[ax], r[ax]]); dd = d.copy(); dd[np.abs(dd) < eps] = 0; s = np.sign(dd)
        for a_, b_, c_ in ((0, 1, 2), (1, 0, 2), (2, 0, 1)):
            if s[a_] != 0 and s[a_] != s[b_] and s[a_] != s[c_]:
                t1 = pp[a_] + (pp[b_] - pp[a_]) * dd[a_] / (dd[a_] - dd[b_]) if s[b_] != 0 else pp[b_]
                t2 = pp[a_] + (pp[c_] - pp[a_]) * dd[a_] / (dd[a_] - dd[c_]) if s[c_] != 0 else pp[c_]
                return min(t1, t2), max(t1, t2)
            if s[a_] == 0 and s[b_] == s[c_] and s[b_] != 0: return pp[a_], pp[a_]
        z = np.flatnonzero(s == 0)
        if len(z) >= 2: return min(pp[z]), max(pp[z])
        return None
    i1 = interval(p1, q1, r1, d1); i2 = interval(p2, q2, r2, d2)
    if i1 is None or i2 is None: return False
    return not (i1[1] < i2[0] - eps or i2[1] < i1[0] - eps)


def self_intersections(P, T, candidates=None, merge_groups=(), chunk=256):
    """Non-adjacent triangle pairs that actually intersect. Adjacency is decided on merged vertex ids so seam
    duplicates do not produce false 'touching' pairs. `candidates` restricts the first triangle of each pair
    (the second ranges over all). Returns sorted list of (i, j), i < j."""
    P = np.asarray(P, float); T = np.asarray(T); n = len(P); canon = np.arange(n)
    for ids in merge_groups:
        for i in ids: canon[i] = ids[0]
    Tc = canon[T]; lo = P[T].min(1); hi = P[T].max(1)
    cand = np.arange(len(T)) if candidates is None else np.asarray(candidates, int); pairs = set()
    for s in range(0, len(cand), chunk):
        ci = cand[s:s + chunk]
        ov = (lo[ci][:, None, :] <= hi[None, :, :]).all(2) & (hi[ci][:, None, :] >= lo[None, :, :]).all(2)
        share = (Tc[ci][:, None, :, None] == Tc[None, :, None, :]).any((2, 3)); ov &= ~share
        for a, b in zip(*np.nonzero(ov)):
            i, j = int(ci[a]), int(b)
            if i == j or (candidates is None and j < i) or (min(i, j), max(i, j)) in pairs: continue
            if _tri_tri_intersect(P[T[i, 0]], P[T[i, 1]], P[T[i, 2]], P[T[j, 0]], P[T[j, 1]], P[T[j, 2]]):
                pairs.add((min(i, j), max(i, j)))
    return sorted(pairs)


def outer_shell_faces(P, T, eps=1e-4):
    """Faces whose outward normal ray from the centroid hits no other face: the visible skin (mouth bag, nostril
    and socket interiors are occluded). Computed once on the untouched donor; brute-force, ~4 s for 6k faces."""
    from .geometry import TriangleSoup
    P = np.asarray(P, float); T = np.asarray(T); soup = TriangleSoup(P, T); fn = face_normals(P, T); cen = P[T].mean(1)
    return np.array([soup.ray_hit(cen[i] + fn[i] * eps, fn[i]) is None for i in range(len(T))])


def face_metrics(P, T, merge_groups=(), sliver_deg=3.0, fold_dot=0.0, dihedral_deg=150.0):
    """Raw metrics and flags for one mesh state (no reference needed)."""
    area = face_areas(P, T); ang = min_angles_deg(P, T); fold = fold_score(P, T, merge_groups)
    dih = max_dihedral_deg(P, T, merge_groups)
    return {'area': area, 'min_angle_deg': ang, 'fold_dot': fold, 'max_dihedral_deg': dih,
            'sliver': ang < sliver_deg, 'inverted_local': fold < fold_dot, 'folded_edge': dih > dihedral_deg}


def classify_faces(before, after, T, merge_groups=(), area_collapse_ratio=0.05, **kw):
    """Flags on `after` and the same flags on `before` so callers can separate inherited from introduced defects.
    Adds 'collapsed' (area fell below `area_collapse_ratio` of before) and 'flip_vs_before'."""
    T = np.asarray(T); out = {'before': face_metrics(before, T, merge_groups, **kw), 'after': face_metrics(after, T, merge_groups, **kw)}
    a0 = out['before']['area']; a1 = out['after']['area']
    out['after']['collapsed'] = (a0 > 1e-12) & (a1 < area_collapse_ratio * a0); out['before']['collapsed'] = np.zeros(len(T), bool)
    out['flip_vs_before'] = (face_normals(before, T) * face_normals(after, T)).sum(1) < 0
    return out
