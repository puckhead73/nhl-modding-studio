"""Native half-float storage model for Legacy (Xbox 360 RX2) positions and the lip-crossing classifier.

Storage model (matches the pinned writer, Studio `tdb-rx2/src/playermesh.rs::f32_to_half_be`, as invoked by Codex's
`legacy_diagnostic.rs`): the exchange JSON float is cast to float32 (numpy, round-to-nearest-even), then to IEEE
half with the writer's rule - round half UP on the dropped mantissa bit (`round = (man >> 12) & 1`), carry into the
exponent on overflow. This differs from Python `struct.pack('>e')` (round-half-to-even) only on exact ties. The
model is a prediction to be checked against Codex's independent reparse (`verify_model_against_reparsed`); it
never replaces the native repack.

Contents
  half_bits_from_f32 / f32_from_half_bits   vectorised writer/reader
  native_round(x)                            float64 -> float32 -> half -> float64 (predicted stored value)
  half_ulp(x)                                spacing of the half grid at x (float64)
  is_representable(x)                        native_round(x) == x
  grid_translation(P_block, t)               nearest translation to t that keeps every P_block + t representable
  quantize_exchange(ex, roles)               exchange with predicted native positions (haircap untouched by design)
  lip_pairs(...) / classify_pairs(...)       exact tri-tri pairs among lid/lip outer faces and their classification
Frame: native for storage (x lateral, y up, z forward); the classifier works in whatever frame it is handed.
"""
import struct
import numpy as np
from . import face_quality as FQ


# ---- writer / reader -----------------------------------------------------------------------------------------------
def half_bits_from_f32(v):
    """Writer rule (Rust f32_to_half_be), vectorised. v: float32 array -> uint16 bits."""
    shape = np.asarray(v).shape
    # Work as a 1D array: NumPy scalar uint64 shifts reject signed scalar operands.
    bits = np.asarray(v, np.float32).reshape(-1).view(np.uint32).astype(np.uint64)
    sign = (bits >> 16) & 0x8000; exp = ((bits >> 23) & 0xFF).astype(np.int64); man = bits & 0x7FFFFF
    e = exp - 127 + 15; h = np.zeros(bits.shape, np.uint64)
    nan_inf = exp == 0xFF; h[nan_inf] = sign[nan_inf] | 0x7C00 | (man[nan_inf] != 0)
    over = ~nan_inf & (e >= 0x1F); h[over] = sign[over] | 0x7C00
    under = ~nan_inf & (e <= 0) & (e < -10); h[under] = sign[under]
    sub = ~nan_inf & (e <= 0) & (e >= -10)
    if sub.any():
        full = man[sub] | 0x800000; shift = (14 - e[sub]).astype(np.uint64); hm = full >> shift; rd = (full >> (shift - 1)) & 1
        h[sub] = sign[sub] | (hm + rd)
    norm = ~nan_inf & (e > 0) & (e < 0x1F)
    hm = man[norm] >> 13; rd = (man[norm] >> 12) & 1
    h[norm] = sign[norm] | ((((e[norm].astype(np.uint64)) << 10) | hm) + rd)
    return h.astype(np.uint16).reshape(shape)


def f32_from_half_bits(h):
    """Reader rule (Rust half_be), vectorised. uint16 bits -> float64 values (exact)."""
    h = np.asarray(h, np.uint16).astype(np.int64); s = (h >> 15) & 1; e = (h >> 10) & 0x1F; m = h & 0x3FF
    v = np.where(e == 0, (m / 1024.0) * 2.0 ** -14, np.where(e == 31, np.inf, (1.0 + m / 1024.0) * 2.0 ** (e - 15)))
    return np.where(s != 0, -v, v)


def native_round(x):
    """Predicted stored value of float64 exchange coordinates: float32 cast (numpy RNE) then the writer's half rule."""
    return f32_from_half_bits(half_bits_from_f32(np.asarray(x, np.float64).astype(np.float32)))


def python_round(x):
    """Reference: Python struct '>e' (round-half-to-even from float64 directly). Used only to expose tie behaviour."""
    x = np.asarray(x, np.float64); flat = x.ravel()
    return np.array([struct.unpack('>e', struct.pack('>e', float(v)))[0] for v in flat]).reshape(x.shape)


def half_ulp(x):
    """Grid spacing of IEEE half at |x| (normal range); subnormal spacing below 2^-14."""
    a = np.abs(np.asarray(x, np.float64)); e = np.floor(np.log2(np.maximum(a, 2.0 ** -14)))
    return np.where(a < 2.0 ** -14, 2.0 ** -24, 2.0 ** (e - 10))


def is_representable(x, atol=0.0):
    x = np.asarray(x, np.float64); return np.abs(native_round(x) - x) <= atol


def storage_error(x):
    """Per-record |native_round(x) - x| (same units as x)."""
    x = np.asarray(x, np.float64); return np.linalg.norm(native_round(x) - x, axis=-1)


# ---- representable rigid translation --------------------------------------------------------------------------------
def grid_translation(P_block, t, search=3):
    """Nearest translation t' to t (per axis) such that every P_block + t' is exactly representable in native half,
    for a block whose coordinates are themselves representable (donor storage). Per axis the candidate step is the
    coarsest half ulp among the translated block records; multiples k*ulp around round(t/ulp) are tried (|dk| <=
    search) and the closest representable one is chosen. Returns (t', report); report['ok'] False if none found."""
    P_block = np.asarray(P_block, np.float64); t = np.asarray(t, np.float64); tp = t.copy(); rep = {'axes': [], 'ok': True}
    for ax in range(3):
        u = float(half_ulp(P_block[:, ax] + t[ax]).max()); k0 = np.round(t[ax] / u); best = None
        for dk in sorted(range(-search, search + 1), key=abs):
            cand = (k0 + dk) * u; ok = bool(is_representable(P_block[:, ax] + cand).all())
            if ok and (best is None or abs(cand - t[ax]) < abs(best - t[ax])): best = cand
        if best is None: rep['ok'] = False; best = k0 * u
        tp[ax] = best; rep['axes'].append({'axis': ax, 'ulp_m': u, 'requested_m': float(t[ax]), 'chosen_m': float(best), 'adjust_mm': float((best - t[ax]) * 1000), 'k': int(round(best / u)), 'all_representable': bool(is_representable(P_block[:, ax] + best).all())})
    rep['adjust_mm'] = float(np.linalg.norm(tp - t) * 1000); return tp, rep


# ---- exchange-level prediction ------------------------------------------------------------------------------------
def quantize_exchange(ex, roles=('head', 'eye_r', 'eye_l', 'mouthbag')):
    """Copy of the exchange with every listed role's positions replaced by their predicted native storage values
    (native frame). The haircap is not listed because the writer leaves it byte-identical (candidate == donor)."""
    import copy
    from . import legacy_exchange as LX
    out = copy.deepcopy(ex)
    for s in out['submeshes']:
        if s['role'] in roles: s['positions'] = native_round(np.asarray(s['positions'], np.float64)).tolist()
    return out


def verify_model_against_reparsed(ex_candidate, ex_reparsed, roles=('head', 'eye_r', 'eye_l', 'mouthbag', 'haircap')):
    """Predicted native positions vs Codex's independent reparse: per role exact-match count and max mismatch (mm),
    plus how many coordinates would differ under Python round-half-even (tie exposure)."""
    from . import legacy_exchange as LX
    out = {}
    for r in roles:
        C = np.asarray(LX.submesh(ex_candidate, r)['positions'], np.float64); R = np.asarray(LX.submesh(ex_reparsed, r)['positions'], np.float64)
        pred = native_round(C) if r != 'haircap' else C; d = np.abs(pred - R)
        ties = int((half_bits_from_f32(C.astype(np.float32)) != half_bits_from_f32(python_round(C).astype(np.float32))).sum()) if r != 'haircap' else 0
        out[r] = {'records': int(len(C)), 'coords_exact': int((d == 0).sum()), 'coords_total': int(d.size), 'max_mismatch_mm': float(d.max() * 1000), 'coords_where_python_rne_would_differ': ties}
    out['all_exact'] = all(v['coords_exact'] == v['coords_total'] for v in out.values() if isinstance(v, dict)); return out


# ---- lip / lid crossing classifier ------------------------------------------------------------------------------------
def lidlip_faces(mk):
    """Outer-shell faces touching a lid, lip or mouth-corner bucket record (the first triangle of each pair)."""
    b = mk['bucket']; T = mk['T']
    m = (b['upper_eyelid_r'] | b['lower_eyelid_r'] | b['upper_eyelid_l'] | b['lower_eyelid_l'] | b['upper_lip'] | b['lower_lip'] | b['mouth_corner_l'] | b['mouth_corner_r'])
    return np.flatnonzero(m[T].any(1) & mk['outer_faces'])


def lip_pairs(P, T, mk, candidates=None):
    """Exact self-intersection pairs (i<j) with i or j among the lid/lip outer faces (same classifier as
    legacy_qualify.screen_set): non-adjacent on merged ids, Möller interval test, coplanar pairs excluded."""
    cand = lidlip_faces(mk) if candidates is None else np.asarray(candidates, int)
    return [tuple(map(int, p)) for p in FQ.self_intersections(P, T, candidates=cand, merge_groups=mk['dup'])]


def coplanar_pairs(P, T, mk, candidates=None, eps=1e-12):
    """Bbox-overlapping, non-adjacent pairs that the classifier cannot decide (both triangles within eps of the
    other's plane). Reported as unknowns, never as crossings."""
    P = np.asarray(P, float); cand = lidlip_faces(mk) if candidates is None else np.asarray(candidates, int); n = len(P); canon = np.arange(n)
    for ids in mk['dup']:
        for i in ids: canon[i] = ids[0]
    Tc = canon[T]; lo = P[T].min(1); hi = P[T].max(1); out = []
    ov = (lo[cand][:, None, :] <= hi[None, :, :]).all(2) & (hi[cand][:, None, :] >= lo[None, :, :]).all(2)
    share = (Tc[cand][:, None, :, None] == Tc[None, :, None, :]).any((2, 3)); ov &= ~share
    for a, b in zip(*np.nonzero(ov)):
        i, j = int(cand[a]), int(b)
        if i == j: continue
        p1, q1, r1 = P[T[i]]; p2, q2, r2 = P[T[j]]; n1 = np.cross(q1 - p1, r1 - p1); n2 = np.cross(q2 - p2, r2 - p2)
        d2 = np.abs((np.array([p2, q2, r2]) - p1) @ n1); d1 = np.abs((np.array([p1, q1, r1]) - p2) @ n2)
        if (d1 < eps).all() and (d2 < eps).all(): out.append((min(i, j), max(i, j)))
    return sorted(set(out))


def adjacency_exclusions(P, T, mk, candidates=None):
    """Bbox-overlapping pairs excluded by the adjacency rule (share a merged vertex): they touch by construction and
    are never counted as crossings. Count only."""
    P = np.asarray(P, float); cand = lidlip_faces(mk) if candidates is None else np.asarray(candidates, int); n = len(P); canon = np.arange(n)
    for ids in mk['dup']:
        for i in ids: canon[i] = ids[0]
    Tc = canon[T]; lo = P[T].min(1); hi = P[T].max(1)
    ov = (lo[cand][:, None, :] <= hi[None, :, :]).all(2) & (hi[cand][:, None, :] >= lo[None, :, :]).all(2)
    share = (Tc[cand][:, None, :, None] == Tc[None, :, None, :]).any((2, 3)); ex = ov & share; ex[np.arange(len(cand)), cand] = False
    return int(ex.sum())


def classify_pairs(P_ref, P_new, T, mk, pairs_ref, pairs_new, block_ids=(), blend_ids=()):
    """Per-pair records, areas, centroid distance, region (visible lip = both faces outer-shell; interior roll = a
    non-outer face involved), block membership, and status: inherited (in ref and new), introduced (new only),
    removed (ref only). block_ids / blend_ids: record ids of the rigid donor lip block and its blend ring."""
    P_new = np.asarray(P_new, float); P_ref = np.asarray(P_ref, float); outer = mk['outer_faces']; ml = set(int(i) for i in mk['mouth_loop'])
    block = set(int(i) for i in block_ids); blend = set(int(i) for i in blend_ids); rows = []
    for pr in sorted(set(pairs_ref) | set(pairs_new)):
        i, j = pr; recs = sorted(set(int(v) for v in T[i]) | set(int(v) for v in T[j]))
        a_i = FQ.face_areas(P_new, T[[i]])[0] * 1e6; a_j = FQ.face_areas(P_new, T[[j]])[0] * 1e6
        cen = np.linalg.norm(P_new[T[i]].mean(0) - P_new[T[j]].mean(0)) * 1000; moved = float(np.linalg.norm(P_new[recs] - P_ref[recs], axis=1).max() * 1000)
        status = 'inherited' if pr in pairs_ref and pr in pairs_new else ('introduced' if pr in pairs_new else 'removed')
        where = 'visible_lip_both_outer' if outer[i] and outer[j] else 'interior_roll_one_face_not_outer'
        membership = {'block': sum(r in block for r in recs), 'blend': sum(r in blend for r in recs), 'mouth_loop': sum(r in ml for r in recs), 'other': sum((r not in block and r not in blend) for r in recs)}
        rows.append({'pair': [i, j], 'status': status, 'records': recs, 'areas_mm2': [float(a_i), float(a_j)], 'centroid_distance_mm': float(cen), 'max_record_move_vs_ref_mm': moved, 'region': where, 'membership': membership})
    return rows


# ---- crossing-driven correction, evaluated on predicted native coordinates -----------------------------------------
def repair_block_crossings(fitp, P_donor, T, mk, block_ids, movable_ids, to_native, from_native, max_iters=8, step_scale=1.0):
    """Bounded neighbourhood adjustment chosen from the measured cause: only records of `movable_ids` that take part in
    an actual crossing between a rigid-block face (all three records in `block_ids`) and a non-block face, measured on
    the PREDICTED native coordinates, are moved. Each such record is pushed along the block face's (donor) normal
    towards the donor's side of that plane by its own storage error bound (0.5 x |half-ulp vector|, `step_scale` x).
    Inherited donor crossings (present in the untouched donor) are never targets. Seam groups are re-merged after each
    pass. Returns (fitp, report); report['resolved'] is False when crossings remain after `max_iters`."""
    fitp = np.asarray(fitp, float).copy(); P_donor = np.asarray(P_donor, float); T = np.asarray(T); n = len(fitp)
    isb = np.zeros(n, bool); isb[np.asarray(block_ids, int)] = True; mov = np.zeros(n, bool); mov[np.asarray(movable_ids, int)] = True
    bfaces = np.flatnonzero(isb[T].all(1)); nb_ids = np.unique(T[np.isin(T, np.flatnonzero(mov)).any(1)]); nb_faces = np.flatnonzero(np.isin(T, nb_ids).any(1))
    fn_d = FQ.face_normals(P_donor, T); donor_pairs = set(map(tuple, FQ.self_intersections(P_donor, T, candidates=nb_faces, merge_groups=mk['dup'])))
    def block_pairs(Pq):
        prs = FQ.self_intersections(Pq, T, candidates=nb_faces, merge_groups=mk['dup'])
        return [tuple(map(int, pq)) for pq in prs if (isb[T[pq[0]]].all() != isb[T[pq[1]]].all()) and tuple(map(int, pq)) not in donor_pairs]
    pushed = {}; history = []
    for it in range(max_iters + 1):
        Pq = from_native(native_round(to_native(fitp))); targets = block_pairs(Pq); history.append(len(targets))
        if not targets or it == max_iters: break
        for i, j in targets:
            f, g = (i, j) if isb[T[i]].all() else (j, i); nrm = fn_d[f]; a_d = P_donor[T[f, 0]]; a_f = fitp[T[f, 0]]
            for r in T[g]:
                if not mov[r]: continue
                s = np.sign(float((P_donor[r] - a_d) @ nrm)); s = s if s != 0 else 1.0
                step = 0.5 * float(np.linalg.norm(half_ulp(to_native(fitp[r][None])[0]))) * step_scale
                fitp[r] = fitp[r] + nrm * s * step; pushed[int(r)] = pushed.get(int(r), 0.0) + step * 1000
        for ids in mk['dup']: fitp[ids] = fitp[ids].mean(0)
    return fitp, {'policy': 'crossing_driven', 'iterations': len(history) - 1, 'block_crossings_per_iteration_predicted_native': history, 'resolved': history[-1] == 0, 'records_pushed': len(pushed), 'max_push_mm': max(pushed.values(), default=0.0), 'pushed_mm': {str(k): round(v, 4) for k, v in sorted(pushed.items())}, 'inherited_donor_pairs_in_neighbourhood': len(donor_pairs)}


# ---- independent triangle-triangle test (edge-vs-triangle ray casts, not the Moller interval test) ---------------
def _seg_hits_tri(p0, p1, a, b, c, eps=1e-12):
    d = p1 - p0; e1 = b - a; e2 = c - a; h = np.cross(d, e2); det = e1 @ h
    if abs(det) < eps: return False
    inv = 1.0 / det; s = p0 - a; u = inv * (s @ h)
    if u < -eps or u > 1 + eps: return False
    q = np.cross(s, e1); v = inv * (d @ q)
    if v < -eps or u + v > 1 + eps: return False
    t = inv * (e2 @ q); return -eps <= t <= 1 + eps


def independent_tri_tri(p1, q1, r1, p2, q2, r2):
    """Two triangles cross iff an edge of one passes through the other (segment/triangle Moller-Trumbore). Coplanar
    pairs return False (parallel edges never register), matching the primary classifier's exclusion."""
    A = (p1, q1, r1); B = (p2, q2, r2)
    for (s0, s1) in ((A[0], A[1]), (A[1], A[2]), (A[2], A[0])):
        if _seg_hits_tri(s0, s1, *B): return True
    for (s0, s1) in ((B[0], B[1]), (B[1], B[2]), (B[2], B[0])):
        if _seg_hits_tri(s0, s1, *A): return True
    return False


def independent_pairs(P, T, candidates, merge_groups=()):
    """Same candidate/adjacency rules as face_quality.self_intersections, different geometric predicate."""
    P = np.asarray(P, float); T = np.asarray(T); n = len(P); canon = np.arange(n)
    for ids in merge_groups:
        for i in ids: canon[i] = ids[0]
    Tc = canon[T]; lo = P[T].min(1); hi = P[T].max(1); cand = np.asarray(candidates, int); out = set()
    for s in range(0, len(cand), 256):
        ci = cand[s:s + 256]; ov = (lo[ci][:, None, :] <= hi[None, :, :]).all(2) & (hi[ci][:, None, :] >= lo[None, :, :]).all(2)
        share = (Tc[ci][:, None, :, None] == Tc[None, :, None, :]).any((2, 3)); ov &= ~share
        for a, b in zip(*np.nonzero(ov)):
            i, j = int(ci[a]), int(b)
            if i == j or (min(i, j), max(i, j)) in out: continue
            if independent_tri_tri(*P[T[i]], *P[T[j]]): out.add((min(i, j), max(i, j)))
    return sorted(out)
