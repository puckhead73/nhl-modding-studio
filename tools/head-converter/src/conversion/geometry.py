"""Shared mesh helpers (numpy only)."""
import collections
import numpy as np


def native_to_blender(a):
    a = np.asarray(a, float); return np.c_[a[:, 0], -a[:, 2], a[:, 1]]


def blender_to_native(a):
    a = np.asarray(a, float); return np.c_[a[:, 0], a[:, 2], -a[:, 1]]


def seam_groups(positions, decimals=6):
    """Groups of vertex ids sharing an identical position (native split records)."""
    groups = {}
    for i, p in enumerate(np.round(np.asarray(positions, float), decimals)):
        groups.setdefault(tuple(p), []).append(i)
    return [ids for ids in groups.values() if len(ids) > 1]


def adjacency(n, tris, merge_groups=()):
    """Vertex adjacency sets; optionally merged across seam groups so graph operations ignore UV seams."""
    adj = [set() for _ in range(n)]
    for a, b, c in np.asarray(tris):
        adj[a].update((b, c)); adj[b].update((a, c)); adj[c].update((a, b))
    for ids in merge_groups:
        union = set().union(*(adj[i] for i in ids)) - set(ids)
        for i in ids: adj[i] = set(union)
    return adj


def open_boundary_vertices(tris):
    ec = collections.Counter()
    for a, b, c in np.asarray(tris):
        for e in ((a, b), (b, c), (c, a)): ec[(min(e), max(e))] += 1
    return np.array(sorted({v for e, k in ec.items() if k == 1 for v in e}), int)


def bottom_loop(positions, tris, fraction=0.25):
    """Open-boundary vertices in the lowest `fraction` of the open set's z range (excludes eye/mouth holes)."""
    P = np.asarray(positions, float); ov = open_boundary_vertices(tris)
    if len(ov) == 0: return ov
    z = P[ov, 2]; return ov[z < z.min() + fraction * (z.max() - z.min())]


def grow(ids, adj, rings):
    s = set(int(i) for i in ids)
    for _ in range(rings): s |= {j for i in list(s) for j in adj[i]}
    return np.array(sorted(s), int)


def ring_distance(seeds, adj, n, max_rings):
    d = np.full(n, np.inf); d[list(seeds)] = 0; frontier = set(int(i) for i in seeds)
    for k in range(1, max_rings + 1):
        frontier = {j for i in frontier for j in adj[i] if d[j] == np.inf}
        for j in frontier: d[j] = k
    return d


def vertex_normals(positions, tris, merge_groups=()):
    P = np.asarray(positions, float); T = np.asarray(tris)
    fn = np.cross(P[T[:, 1]] - P[T[:, 0]], P[T[:, 2]] - P[T[:, 0]])
    vn = np.zeros_like(P)
    for k in range(3): np.add.at(vn, T[:, k], fn)
    for ids in merge_groups: vn[ids] = vn[ids].sum(0)
    return vn / np.maximum(np.linalg.norm(vn, axis=1)[:, None], 1e-20)


class TriangleSoup:
    """Brute-force ray/nearest queries against a triangle mesh (fine for ~10k triangles)."""

    def __init__(self, positions, tris):
        P = np.asarray(positions, float); T = np.asarray(tris)
        self.A, self.B, self.C = P[T[:, 0]], P[T[:, 1]], P[T[:, 2]]
        self.E1 = self.B - self.A; self.E2 = self.C - self.A

    def ray_hit(self, o, d):
        """Möller–Trumbore; nearest positive t along d from o, or None."""
        P = np.cross(d, self.E2); det = (self.E1 * P).sum(1); ok = np.abs(det) > 1e-12
        inv = np.where(ok, 1.0 / np.where(ok, det, 1), 0); T = o - self.A; u = (T * P).sum(1) * inv
        Q = np.cross(T, self.E1); v = (d * Q).sum(1) * inv; t = (self.E2 * Q).sum(1) * inv
        hit = ok & (u >= 0) & (v >= 0) & (u + v <= 1) & (t > 1e-6)
        return float(t[hit].min()) if hit.any() else None

    def nearest(self, p):
        """Closest point on the surface to p and its distance (Ericson point-triangle)."""
        A, B, C, E1, E2 = self.A, self.B, self.C, self.E1, self.E2
        AP = p - A; d1 = (E1 * AP).sum(1); d2 = (E2 * AP).sum(1)
        BP = p - B; d3 = (E1 * BP).sum(1); d4 = (E2 * BP).sum(1)
        CP = p - C; d5 = (E1 * CP).sum(1); d6 = (E2 * CP).sum(1)
        va = d3 * d6 - d5 * d4; vb = d5 * d2 - d1 * d6; vc = d1 * d4 - d3 * d2
        denom = 1.0 / (va + vb + vc + 1e-30); v = vb * denom; w = vc * denom
        Q = A + E1 * v[:, None] + E2 * w[:, None]
        Q = np.where(((d1 <= 0) & (d2 <= 0))[:, None], A, Q); Q = np.where(((d3 >= 0) & (d4 <= d3))[:, None], B, Q); Q = np.where(((d6 >= 0) & (d5 <= d6))[:, None], C, Q)
        e1 = (vc <= 0) & (d1 >= 0) & (d3 <= 0); t1 = d1 / (d1 - d3 + 1e-30); Q = np.where(e1[:, None], A + E1 * t1[:, None], Q)
        e2 = (vb <= 0) & (d2 >= 0) & (d6 <= 0); t2 = d2 / (d2 - d6 + 1e-30); Q = np.where(e2[:, None], A + E2 * t2[:, None], Q)
        e3 = (va <= 0) & ((d4 - d3) >= 0) & ((d5 - d6) >= 0); t3 = (d4 - d3) / ((d4 - d3) + (d5 - d6) + 1e-30); Q = np.where(e3[:, None], B + (C - B) * t3[:, None], Q)
        dd = ((p - Q) ** 2).sum(1); k = int(np.argmin(dd)); return Q[k], float(np.sqrt(dd[k]))

    def nearest_face(self, p):
        """Like nearest() but also returns the index of the triangle carrying the closest point."""
        q, d = self.nearest(p)
        dd = ((q - self.A) ** 2).sum(1) + ((q - self.B) ** 2).sum(1) + ((q - self.C) ** 2).sum(1)
        # exact: recompute closest points is expensive; instead pick, among faces whose plane distance to p equals d,
        # the one containing q (barycentric test); fall back to smallest vertex-distance sum.
        E1, E2 = self.E1, self.E2; w = q - self.A
        d00 = (E1 * E1).sum(1); d01 = (E1 * E2).sum(1); d11 = (E2 * E2).sum(1); d20 = (w * E1).sum(1); d21 = (w * E2).sum(1)
        den = d00 * d11 - d01 * d01 + 1e-30; v = (d11 * d20 - d01 * d21) / den; u = (d00 * d21 - d01 * d20) / den
        inside = (v >= -1e-6) & (u >= -1e-6) & (u + v <= 1 + 1e-6)
        n = np.cross(E1, E2); n /= np.maximum(np.linalg.norm(n, axis=1)[:, None], 1e-20)
        off = np.abs(((q - self.A) * n).sum(1)) < 1e-9
        cand = np.flatnonzero(inside & off)
        k = int(cand[np.argmin(dd[cand])]) if len(cand) else int(np.argmin(dd))
        return q, d, k


def laplace_fill(D, free, nb, iters=2000, tol=1e-6):
    """Gauss-Seidel harmonic interpolation of the rows D[free] from their neighbours (Dirichlet elsewhere)."""
    for it in range(iters):
        delta = 0.0
        for i in free:
            v = D[nb[i]].mean(0); delta = max(delta, float(np.abs(v - D[i]).max())); D[i] = v
        if delta < tol: break
    return it + 1


def triangle_quality(before, after, tris):
    T = np.asarray(tris); b = np.asarray(before); a = np.asarray(after)
    cb = np.cross(b[T[:, 1]] - b[T[:, 0]], b[T[:, 2]] - b[T[:, 0]]); ca = np.cross(a[T[:, 1]] - a[T[:, 0]], a[T[:, 2]] - a[T[:, 0]])
    collapsed = int(((np.linalg.norm(cb, axis=1) > 1e-10) & (np.linalg.norm(ca, axis=1) < 1e-10)).sum())
    flips = int(((cb * ca).sum(1) < 0).sum())
    return {'new_collapsed_triangles': collapsed, 'orientation_flips': flips}
