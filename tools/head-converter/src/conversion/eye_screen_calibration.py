"""Calibration of the three disputed Legacy eye screens against FIXED stored geometry (numpy only; no fitting):
  1. cap/head contact  - per-pair evidence on the ACTUAL stored triangles instead of a crossing count: head-face class
                         (rim margin / outer skin / fold underside / hidden fan), intersection segment, directional intrusion
                         (signed distance of the head face to the cap triangles, negative = toward the cap centre), inside area
                         fraction, vertex-clear-but-crossing, near-coplanar degeneracy, and a disposition (contact / intrusion)
                         whose thresholds are declared parameters; visibility is attached by the caller (alpha-aware ray cast)
  2. orientation       - per inward face: radial dot (kept as a diagnostic), donor-normal dot, the CONTINUOUS candidate's radial dot
                         (storage-created when the continuous solution is not inward), donor-frame deformation gradient (2-D
                         Jacobian: singular values and the orientation sign = local inversion), signed-dihedral change and crease
                         sign flips on its edges, neighbour-normal continuity vs the donor, fold / self-intersection participation
  3. envelope tolerance - numerical evaluation error of the dihedral (float32 round trip, reproducibility) separated from an ALLOWANCE
                         for storage-induced shape change derived per edge from the edge's conditioning: the linearised worst-case
                         angle change when every stored vertex moves by half a storage step per axis (the closest stored point to a
                         continuous solution lies within that box), capped so a badly conditioned edge cannot authorise arbitrary
                         changes; controls decide whether the allowance is usable (it must reject known folds and sign flips)
Historical screens are untouched: this module only adds measurements; the judge selects them behind an explicit policy version.

2026-09-16 correction (docs/LEGACY_EYE_SCREEN_CALIBRATION_REVIEW_2026-09-16.md; policy 'v3_corrected'): the v2 functions above are kept
verbatim for reproducibility of the frozen v2 results; the *_v3 functions below correct their semantics -
  contact_evidence_v3   pair rows carry only pair-specific quantities (segment); face-level quantities (nearest-soup depth, sampled inside
                        fraction) live once per FACE (no duplication across cap faces); the cap footprint is the ACTUAL open cap (a sample is
                        inside the footprint when its ray from the cap centre hits a cap triangle - no circular max-angle mask); samples at the
                        cap centre (zero radial vector) are 'undefined', never 'outside'; the sampled inside fraction is refined
                        (66 -> 231 -> 861 samples) and the level-to-level change is reported as its error estimate; the deepest point is
                        the minimum over vertices and samples = a sampled estimate, not a bound; contact at the cap boundary (samples whose
                        centre ray misses the cap) is flagged 'boundary_unresolved'; per-pair relative quantization sensitivity = the sum of
                        the radial half-ulp components of the head face's and the cap face's records; the 0.49 mm constant is a LABELLED
                        EXPERIMENTAL ALLOWANCE (`EXPERIMENTAL_ALLOWANCE_MM`) used only to name a disposition 'sub_allowance_contact' - never
                        'harmless'; every raw value is kept
  patch_visibility      visibility of the INTRUDING samples themselves per view (ray from the sample toward the camera against the opaque
                        head / caps / mouthbag; a haircap hit is reported as 'behind_haircap_alpha_not_evaluated'; samples on a face that faces
                        away from the camera are 'back_facing'); whole-face pixel counts are a separate, weaker quantity
  orientation_evidence_v3   `donor_facing_sign` replaces the misnamed 'orientation_sign_vs_donor' (sign(n_c . n_d): a donor-facing comparison
                        that a proper 180-degree rigid rotation also flips; NOT an intrinsic inversion); structural labels need CONTEXTUAL
                        evidence only: fold-over pair, self-intersection, local inversion against the adjacent faces (FQ inverted_local),
                        crease sign flip on the face's own edges; storage attribution is 'unknown' unless a genuine pre-quantization snapshot
                        exists (phase labels come from the caller); anatomical-roll labels stay 'provisional'; ambiguous stays blocking
  coordinate_roundtrip_repeat   the former `numerical_error` under its honest name (float32 coordinate round trip + repeat execution)
  edge_sensitivity      documented as a SUM OF FINITE ONE-SIDED PERTURBATIONS (not a derivative, not a certified worst-case interval)
"""
import numpy as np
from . import geometry as G, face_quality as FQ, crease as CR, legacy_quant as LQt

CONTACT_CLASSES = ('rim_margin', 'outer_skin', 'fold_underside', 'hidden_fan')
# declared disposition thresholds (mm): the half-float vertical half-step at the eyes is 0.488 mm - a head face whose deepest point lies
# less than GRID_MM inside the cap surface is indistinguishable from resting on it at storage precision; deeper = intrusion
CONTACT_THRESHOLDS = {'grid_mm': 0.49, 'coplanar_dot': 0.999}


def _seg_tri_point(p0, p1, a, b, c, eps=1e-12):
    d = p1 - p0; e1 = b - a; e2 = c - a; h = np.cross(d, e2); det = e1 @ h
    if abs(det) < eps: return None
    inv = 1.0 / det; s = p0 - a; u = inv * (s @ h)
    if u < -eps or u > 1 + eps: return None
    q = np.cross(s, e1); v = inv * (d @ q)
    if v < -eps or u + v > 1 + eps: return None
    t = inv * (e2 @ q)
    return p0 + t * d if -eps <= t <= 1 + eps else None


def tri_tri_segment(A, B, eps=1e-12):
    """Intersection segment of two triangles (3x3 arrays): the extreme pair among the edge/triangle hits of both. Returns
    (length, p, q) or (0, None, None) when fewer than two hits (touching / coplanar / no crossing)."""
    pts = []
    for X, Y in ((A, B), (B, A)):
        for i in range(3):
            p = _seg_tri_point(X[i], X[(i + 1) % 3], *Y, eps)
            if p is not None: pts.append(p)
    if len(pts) < 2: return 0.0, None, None
    P = np.array(pts); D = np.linalg.norm(P[:, None] - P[None], axis=2); i, j = np.unravel_index(np.argmax(D), D.shape)
    return float(D[i, j]), P[i], P[j]


def head_face_classes(T, lash, outer_faces):
    """Per head face: hidden_fan (all lash), rim_margin (some lash), fold_underside (no lash, not outer), outer_skin."""
    L = lash[T]; fan = L.all(1); band = L.any(1) & ~fan; cls = np.where(fan, 'hidden_fan', np.where(band, 'rim_margin', np.where(outer_faces, 'outer_skin', 'fold_underside')))
    return cls.astype(object)


def signed_to_cap(points, cap_soup, c):
    """Signed distance of points to the actual cap triangles: negative = nearer the cap centre than the nearest cap point."""
    out = np.zeros(len(points))
    for k, p in enumerate(points):
        q, d = cap_soup.nearest(p); out[k] = d * (1 if np.linalg.norm(p - c) >= np.linalg.norm(q - c) else -1)
    return out


def contact_evidence(hp, T, cap_P, cap_T, faces, lash, outer_faces, c_cap, ax, half_deg, thresholds=CONTACT_THRESHOLDS, samples=10):
    """Per-pair evidence for every (head face in `faces`) x (cap triangle) crossing on the stored coordinates (metres in, mm out)."""
    soup = G.TriangleSoup(cap_P, cap_T); cls = head_face_classes(T, lash, outer_faces); nf = FQ.face_normals(hp, T); nc = FQ.face_normals(cap_P, cap_T)
    lo1 = hp[T[faces]].min(1); hi1 = hp[T[faces]].max(1); lo2 = cap_P[cap_T].min(1); hi2 = cap_P[cap_T].max(1); rows = []
    # barycentric samples for the inside-area fraction
    bary = np.array([(i / samples, j / samples, 1 - i / samples - j / samples) for i in range(samples + 1) for j in range(samples + 1 - i)])
    for a, f in enumerate(faces):
        ov = np.flatnonzero((lo1[a] <= hi2).all(1) & (hi1[a] >= lo2).all(1))
        for g in ov:
            A = hp[T[f]]; B = cap_P[cap_T[g]]
            if not FQ._tri_tri_intersect(*A, *B): continue
            L, p, q = tri_tri_segment(A, B)
            sd_v = signed_to_cap(A, soup, c_cap) * 1000; pts = bary @ A; sd_s = signed_to_cap(pts, soup, c_cap) * 1000
            v = pts - c_cap; ang = np.degrees(np.arccos(np.clip((v / np.linalg.norm(v, axis=1)[:, None]) @ ax, -1, 1))); inside = (sd_s < 0) & (ang <= half_deg)
            area = 0.5 * np.linalg.norm(np.cross(A[1] - A[0], A[2] - A[0])) * 1e6; depth = float(min(sd_v.min(), sd_s.min()))
            cop = abs(float(nf[f] @ nc[g])) >= thresholds['coplanar_dot']
            disp = ('coplanar_or_tangent_limit' if cop or L == 0.0 else 'intrusion' if depth < -thresholds['grid_mm'] else 'contact_within_grid')
            rows.append({'head_face': int(f), 'cap_face': int(g), 'class': str(cls[f]), 'records': T[f].tolist(), 'segment_mm': L * 1000, 'segment_mid_native_mm': (G.blender_to_native(((p + q) / 2)[None])[0] * 1000).round(2).tolist() if p is not None else None,
                         'vertex_signed_to_cap_mm': sd_v.round(3).tolist(), 'deepest_mm': depth, 'inside_area_fraction': float(inside.mean()), 'inside_area_mm2': float(inside.mean() * area), 'face_area_mm2': float(area), 'vertex_clear_but_crossing': bool((sd_v >= 0).all()), 'near_coplanar': cop, 'disposition': disp})
    return rows


def summarise_contact(rows, px_of_face):
    """Attach visibility and roll up: per class x disposition counts, visible intrusions, hidden intrusions."""
    for r in rows:
        px = px_of_face.get(r['head_face'], {}); r['visible_px'] = px; r['visible'] = bool(max(px.values(), default=0) >= 20)
        r['reading'] = ('visible_intrusion' if r['disposition'] == 'intrusion' and r['visible'] else 'hidden_intrusion' if r['disposition'] == 'intrusion' else 'visible_contact' if r['visible'] and r['disposition'] == 'contact_within_grid' else 'hidden_contact' if r['disposition'] == 'contact_within_grid' else r['disposition'])
    keys = sorted({(r['class'], r['reading']) for r in rows}); by = {f'{c}|{d}': sum(1 for r in rows if r['class'] == c and r['reading'] == d) for c, d in keys}
    return {'pairs': len(rows), 'by_class_and_reading': by, 'visible_intrusion_pairs': [r for r in rows if r['reading'] == 'visible_intrusion'], 'hidden_intrusion_pairs': sum(1 for r in rows if r['reading'] == 'hidden_intrusion'), 'vertex_clear_but_crossing': sum(1 for r in rows if r['vertex_clear_but_crossing']), 'near_coplanar': sum(1 for r in rows if r['near_coplanar']),
            'deepest_mm': min((r['deepest_mm'] for r in rows), default=0.0), 'max_segment_mm': max((r['segment_mm'] for r in rows), default=0.0), 'inside_area_mm2_total': sum(r['inside_area_mm2'] for r in rows), 'faces_involved': sorted({r['head_face'] for r in rows})}


# ---- orientation ----------------------------------------------------------------------------------------------------
def face_jacobian(P_d, P_c, n_d):
    """2-D deformation gradient donor triangle -> candidate triangle in the donor face frame. Returns (singular values,
    orientation sign relative to the donor normal: -1 = the candidate triangle is mirrored / locally inverted)."""
    e1 = P_d[1] - P_d[0]; e2 = P_d[2] - P_d[0]; u = e1 / max(np.linalg.norm(e1), 1e-12); w = np.cross(n_d, u); Bd = np.array([[e1 @ u, e2 @ u], [e1 @ w, e2 @ w]])
    f1 = P_c[1] - P_c[0]; f2 = P_c[2] - P_c[0]; n_c = np.cross(f1, f2); n_c = n_c / max(np.linalg.norm(n_c), 1e-12); uc = f1 / max(np.linalg.norm(f1), 1e-12); wc = np.cross(n_c, uc); Bc = np.array([[f1 @ uc, f2 @ uc], [f1 @ wc, f2 @ wc]])
    Fm = Bc @ np.linalg.inv(Bd) if abs(np.linalg.det(Bd)) > 1e-18 else np.full((2, 2), np.nan); sv = np.linalg.svd(Fm, compute_uv=False) if np.isfinite(Fm).all() else np.array([np.nan, np.nan])
    return sv, int(np.sign(n_c @ n_d)) if np.isfinite(n_c).all() else 0


def orientation_evidence(f, P, X_stored, X_cont, T, adj_faces, c_cap, crease_edges, fold_faces, selfx_faces, ES, th_d, is_crease, env_lo, env_hi):
    """Evidence for one inward-flagged head face. X_cont may be None (no continuous candidate available)."""
    nd = FQ.face_normals(P, T[[f]])[0]; ns = FQ.face_normals(X_stored, T[[f]])[0]; cen = X_stored[T[f]].mean(0); od = (cen - c_cap) / max(np.linalg.norm(cen - c_cap), 1e-12)
    out = {'face': int(f), 'records': T[f].tolist(), 'radial_dot_stored': float(ns @ od), 'donor_normal_dot': float(ns @ nd), 'area_mm2': {'donor': float(0.5 * np.linalg.norm(np.cross(P[T[f, 1]] - P[T[f, 0]], P[T[f, 2]] - P[T[f, 0]])) * 1e6), 'stored': float(0.5 * np.linalg.norm(np.cross(X_stored[T[f, 1]] - X_stored[T[f, 0]], X_stored[T[f, 2]] - X_stored[T[f, 0]])) * 1e6)}}
    if X_cont is not None:
        nc = FQ.face_normals(X_cont, T[[f]])[0]; cc = X_cont[T[f]].mean(0); odc = (cc - c_cap) / max(np.linalg.norm(cc - c_cap), 1e-12); out['radial_dot_continuous'] = float(nc @ odc); out['storage_created'] = bool(out['radial_dot_continuous'] >= 0 > out['radial_dot_stored'])
    else: out['radial_dot_continuous'] = None; out['storage_created'] = None
    sv, sgn = face_jacobian(P[T[f]], X_stored[T[f]], nd); out['jacobian'] = {'singular_values': [float(x) for x in sv], 'orientation_sign_vs_donor': sgn, 'locally_inverted': sgn < 0}
    nb = adj_faces.get(int(f), []); out['neighbour_continuity_deg'] = {'stored': float(np.degrees(np.arccos(np.clip(ns @ (lambda m: m / max(np.linalg.norm(m), 1e-12))(FQ.face_normals(X_stored, T[nb]).sum(0)), -1, 1)))) if nb else None, 'donor': float(np.degrees(np.arccos(np.clip(nd @ (lambda m: m / max(np.linalg.norm(m), 1e-12))(FQ.face_normals(P, T[nb]).sum(0)), -1, 1)))) if nb else None}
    # edges of this face in the crease edge set: dihedral stored vs donor, sign flips on crease edges, envelope excess
    ed = np.flatnonzero((ES['f'] == f) | (ES['g'] == f)) if ES['n'] else np.zeros(0, int); th_s = CR.signed_dihedrals(X_stored * 1000, T, ES) if ES['n'] else np.zeros(0)
    out['edges'] = [{'edge': [int(ES['a'][i]), int(ES['b'][i])], 'donor_deg': float(np.degrees(th_d[i])), 'stored_deg': float(np.degrees(th_s[i])), 'crease_edge': bool(is_crease[i]), 'sign_flip': bool(is_crease[i] and np.sign(th_s[i]) != np.sign(th_d[i])), 'outside_envelope_deg': float(np.degrees(max(0, th_s[i] - env_hi[i]) + min(0, th_s[i] - env_lo[i])))} for i in ed]
    out['in_fold_pair'] = int(f) in fold_faces; out['in_self_intersection'] = int(f) in selfx_faces
    structural = out['in_fold_pair'] or out['in_self_intersection'] or out['jacobian']['locally_inverted'] or any(e['sign_flip'] for e in out['edges'])
    disc = out['neighbour_continuity_deg']; discont = disc['stored'] is not None and disc['stored'] > disc['donor'] + 60
    if structural or discont: out['disposition'] = 'confirmed_structural'
    elif out['storage_created']: out['disposition'] = 'storage_created_orientation'
    elif out['donor_normal_dot'] >= 0.7 and all(abs(e['outside_envelope_deg']) < 1e-9 for e in out['edges']) and out['radial_dot_stored'] > -0.35: out['disposition'] = 'anatomical_roll_candidate'
    else: out['disposition'] = 'ambiguous'
    return out


# ---- tolerance ----------------------------------------------------------------------------------------------------------
def edge_sensitivity(X_stored_native, T, ES, half_step=True):
    """Per edge: SUM OF FINITE ONE-SIDED PERTURBATIONS |d theta| (deg) over the 12 record-axis moves of +(half) storage step on the
    4 records of the edge (a finite-difference proxy for the storage box the lattice stage can land in - not an analytic derivative and
    not a certified worst-case interval: opposite-sign moves and combined moves are not enumerated), plus the conditioning proxy
    step / min face height. X in native metres."""
    Xb = G.native_to_blender(X_stored_native) * 1000; th0 = CR.signed_dihedrals(Xb, T, ES); ulp = LQt.half_ulp(X_stored_native) * 1000 * (0.5 if half_step else 1.0); n = ES['n']; sens = np.zeros(n); cond = np.zeros(n)
    for i in range(n):
        recs = [int(ES['a'][i]), int(ES['b'][i]), int(ES['c'][i]), int(ES['d'][i])]; tot = 0.0
        for r in recs:
            for ax in range(3):
                Xn = X_stored_native.copy(); Xn[r, ax] += ulp[r, ax] / 1000; th = CR.signed_dihedrals(G.native_to_blender(Xn) * 1000, T, {k: (v[[i]] if isinstance(v, np.ndarray) else v) for k, v in ES.items()} | {'n': 1})[0]
                d = np.arctan2(np.sin(th - th0[i]), np.cos(th - th0[i])); tot += abs(d)
        sens[i] = np.degrees(tot)
        for fid in (ES['f'][i], ES['g'][i]):
            t = T[fid]; A = 0.5 * np.linalg.norm(np.cross(Xb[t[1]] - Xb[t[0]], Xb[t[2]] - Xb[t[0]])); e = np.linalg.norm(Xb[ES['b'][i]] - Xb[ES['a'][i]]); h = 2 * A / max(e, 1e-9); cond[i] = max(cond[i], np.degrees(ulp[recs].max() / max(h, 1e-9)))
    return sens, cond


def proposed_allowance(sens_deg, cap_deg):
    """Per-edge allowance = min(sensitivity, cap). The cap keeps a badly conditioned edge from authorising arbitrary change; the
    fraction of edges hitting the cap is reported by the caller."""
    return np.minimum(np.asarray(sens_deg, float), float(cap_deg))


def numerical_error(X, T, ES):
    """Dihedral evaluation error: float32 round trip of the coordinates and a permuted-evaluation reproducibility check (deg)."""
    th = CR.signed_dihedrals(X, T, ES); th32 = CR.signed_dihedrals(X.astype(np.float32).astype(np.float64), T, ES); th_b = CR.signed_dihedrals(X + 0.0, T, ES)
    d32 = np.degrees(np.abs(np.arctan2(np.sin(th32 - th), np.cos(th32 - th)))); return {'float32_roundtrip_max_deg': float(d32.max()) if len(d32) else 0.0, 'float32_roundtrip_p95_deg': float(np.percentile(d32, 95)) if len(d32) else 0.0, 'reproducibility_max_deg': float(np.degrees(np.abs(th_b - th)).max()) if len(th) else 0.0}


# ====================================================================================================================================
# 2026-09-16 correction: v3 measurements (see module docstring). The v2 functions above are untouched.
EXPERIMENTAL_ALLOWANCE_MM = 0.49          # labelled experimental reporting allowance (half the 0.977 mm vertical step); NOT a proof of harmlessness
SAMPLE_LEVELS = (10, 20, 40)              # barycentric refinement levels (66, 231, 861 samples)


def _bary_grid(n):
    return np.array([(i / n, j / n, 1 - i / n - j / n) for i in range(n + 1) for j in range(n + 1 - i)])


def footprint_hits(points, cap_soup, c):
    """For each point: does the ray from the cap centre through the point hit an actual cap triangle (True), miss it (False = outside the
    open cap's footprint), or is the point at the centre (None = undefined; never classified as outside)."""
    out = []
    for p in points:
        v = p - c; n = np.linalg.norm(v)
        if n < 1e-9: out.append(None); continue
        out.append(cap_soup.ray_hit(c, v / n) is not None)
    return out


def signed_to_cap_safe(points, cap_soup, c):
    """Signed distance to the cap triangles with the centre case explicit: NaN for a point at the centre (no radial direction)."""
    out = np.full(len(points), np.nan)
    for k, p in enumerate(points):
        q, d = cap_soup.nearest(p); n = np.linalg.norm(p - c)
        if n < 1e-9: continue
        out[k] = d * (1 if n >= np.linalg.norm(q - c) else -1)
    return out


def face_contact_v3(A, cap_soup, c, levels=SAMPLE_LEVELS):
    """Face-level quantities for ONE head triangle A (3x3, metres): vertex signed distances, refined sampled inside fraction (samples inside
    the cap surface AND within the actual footprint), boundary / undefined sample counts, deepest sampled point (estimate). mm out."""
    sd_v = signed_to_cap_safe(A, cap_soup, c) * 1000; area = 0.5 * np.linalg.norm(np.cross(A[1] - A[0], A[2] - A[0])) * 1e6; lv = []
    for n in levels:
        pts = _bary_grid(n) @ A; sd = signed_to_cap_safe(pts, cap_soup, c) * 1000; fp = footprint_hits(pts, cap_soup, c)
        inside = np.array([(s_ < 0) and (f_ is True) for s_, f_ in zip(sd, fp)]); undefined = sum(1 for f_ in fp if f_ is None) + int(np.isnan(sd).sum()); boundary = sum(1 for s_, f_ in zip(sd, fp) if (s_ < 0) and (f_ is False))
        lv.append({'samples': int(len(pts)), 'inside_fraction': float(inside.mean()), 'inside_area_mm2': float(inside.mean() * area), 'deepest_sampled_mm': float(np.nanmin(sd)) if np.isfinite(sd).any() else None, 'undefined_samples': int(undefined), 'inside_but_outside_footprint_samples': int(boundary)})
    est = lv[-1]['inside_area_mm2']; err = abs(lv[-1]['inside_area_mm2'] - lv[-2]['inside_area_mm2']) if len(lv) > 1 else None
    deep = [x['deepest_sampled_mm'] for x in lv if x['deepest_sampled_mm'] is not None] + [float(np.nanmin(sd_v))] if np.isfinite(sd_v).any() else [x['deepest_sampled_mm'] for x in lv if x['deepest_sampled_mm'] is not None]
    return {'face_area_mm2': float(area), 'vertex_signed_to_cap_mm': [None if np.isnan(x) else float(x) for x in sd_v], 'levels': lv, 'inside_area_mm2_estimate': est, 'inside_area_level_change_mm2': err, 'deepest_estimate_mm': float(min(deep)) if deep else None,
            'deepest_note': 'minimum over vertices and barycentric samples: a sampled estimate, not a certified bound', 'boundary_unresolved': bool(lv[-1]['inside_but_outside_footprint_samples'] > 0), 'undefined_samples': int(lv[-1]['undefined_samples'])}


def radial_quantization_sensitivity_mm(X_native_head_face, X_native_cap_face, c_native):
    """Relative cap/skin quantization sensitivity for one pair: sum over the two faces of the maximum radial component of their records'
    half-ulp boxes (both meshes are stored on the same anisotropic half-float grid; orientation-dependent). mm."""
    def part(Xn):
        h = LQt.half_ulp(np.asarray(Xn, float)) * 0.5; v = np.asarray(Xn, float) - np.asarray(c_native, float); u = v / np.maximum(np.linalg.norm(v, axis=1), 1e-12)[:, None]; return float((np.abs(u) * h).sum(1).max() * 1000)
    return part(X_native_head_face) + part(X_native_cap_face)


def contact_evidence_v3(hp, T, cap_P, cap_T, faces, lash, outer_faces, c_cap, allowance_mm=EXPERIMENTAL_ALLOWANCE_MM, coplanar_dot=0.999):
    """Per-pair rows (pair-specific: segment, near-coplanar, relative quantization sensitivity) + one face-level record per intersecting head
    face (depth / area / footprint), deduplicated. Dispositions per FACE: 'sub_allowance_contact' (deepest estimate >= -allowance, labelled
    experimental), 'intrusion' (deeper), 'boundary_unresolved' / 'undefined' flags attached; tangent pairs (< 2 hit points) listed as
    'tangent_or_coplanar_unresolved'. Positions in metres (Blender); the native frame for quantization is derived here."""
    soup = G.TriangleSoup(cap_P, cap_T); cls = head_face_classes(T, lash, outer_faces); nf = FQ.face_normals(hp, T); nc = FQ.face_normals(cap_P, cap_T)
    lo1 = hp[T[faces]].min(1); hi1 = hp[T[faces]].max(1); lo2 = cap_P[cap_T].min(1); hi2 = cap_P[cap_T].max(1); pairs = []; face_rows = {}
    c_nat = G.blender_to_native(np.asarray(c_cap, float)[None])[0]
    for a, f in enumerate(faces):
        ov = np.flatnonzero((lo1[a] <= hi2).all(1) & (hi1[a] >= lo2).all(1))
        for g in ov:
            A = hp[T[f]]; B = cap_P[cap_T[g]]
            if not FQ._tri_tri_intersect(*A, *B): continue
            L, p, q = tri_tri_segment(A, B); cop = abs(float(nf[f] @ nc[g])) >= coplanar_dot
            pairs.append({'head_face': int(f), 'cap_face': int(g), 'class': str(cls[f]), 'segment_mm': L * 1000, 'segment_mid_native_mm': (G.blender_to_native(((p + q) / 2)[None])[0] * 1000).round(2).tolist() if p is not None else None, 'near_coplanar': cop,
                          'pair_kind': 'tangent_or_coplanar_unresolved' if (cop or L == 0.0) else 'crossing', 'relative_quantization_sensitivity_mm': radial_quantization_sensitivity_mm(G.blender_to_native(A), G.blender_to_native(B), c_nat)})
            if int(f) not in face_rows:
                fc = face_contact_v3(A, soup, c_cap); deep = fc['deepest_estimate_mm']
                fc.update({'head_face': int(f), 'class': str(cls[f]), 'records': T[f].tolist(), 'cap_faces': [], 'disposition': ('undefined' if deep is None else 'intrusion' if deep < -allowance_mm else 'sub_allowance_contact'), 'allowance_mm_experimental': allowance_mm, 'vertex_clear_but_crossing': bool(all(x is not None and x >= 0 for x in fc['vertex_signed_to_cap_mm']))})
                face_rows[int(f)] = fc
            face_rows[int(f)]['cap_faces'].append(int(g))
    return pairs, face_rows


def patch_visibility(samples_inside, face_normal, view_dirs, opaque_soups, haircap_soup=None):
    """Visibility of the intruding samples per view: 'visible_unoccluded' (ray from the sample toward the camera hits nothing opaque and the
    face fronts the camera), 'behind_haircap_alpha_not_evaluated' (only a haircap triangle in front), 'occluded_by_opaque', 'back_facing',
    'unsampled' (no intruding sample). view_dirs = {name: direction the camera sits along (unit, from the focus toward the camera)}."""
    out = {}
    for name, d in view_dirs.items():
        d = np.asarray(d, float); d = d / np.linalg.norm(d)
        if len(samples_inside) == 0: out[name] = {'state': 'unsampled', 'samples': 0}; continue
        if float(np.asarray(face_normal) @ d) <= 0: out[name] = {'state': 'back_facing', 'samples': int(len(samples_inside))}; continue
        vis = 0; hair = 0; occ = 0
        for p in samples_inside:
            o = p + d * 1e-5; blocked = any(s_.ray_hit(o, d) is not None for s_ in opaque_soups)
            if blocked: occ += 1
            elif haircap_soup is not None and haircap_soup.ray_hit(o, d) is not None: hair += 1
            else: vis += 1
        out[name] = {'state': 'visible_unoccluded' if vis else 'behind_haircap_alpha_not_evaluated' if hair else 'occluded_by_opaque', 'samples': int(len(samples_inside)), 'visible_samples': vis, 'behind_haircap_samples': hair, 'occluded_samples': occ}
    return out


def orientation_evidence_v3(f, P, X_stored, X_pre, T, adj_faces, c_cap, fold_faces, selfx_faces, inverted_local_faces, ES, th_d, is_crease, env_lo, env_hi, pre_phase):
    """Corrected orientation evidence for one inward-flagged face. X_pre = a preserved geometry of an EARLIER phase (or None) with its
    declared phase label `pre_phase` ('pre_quantization_continuous' | 'post_lattice_candidate' | 'none'); storage attribution is
    'storage_created' / 'present_before_storage' only when the phase is genuinely pre-quantization, else 'unknown'."""
    nd = FQ.face_normals(P, T[[f]])[0]; ns = FQ.face_normals(X_stored, T[[f]])[0]; cen = X_stored[T[f]].mean(0); od = (cen - c_cap) / max(np.linalg.norm(cen - c_cap), 1e-12)
    out = {'face': int(f), 'records': T[f].tolist(), 'radial_dot_stored': float(ns @ od), 'donor_normal_dot': float(ns @ nd), 'area_mm2': {'donor': float(0.5 * np.linalg.norm(np.cross(P[T[f, 1]] - P[T[f, 0]], P[T[f, 2]] - P[T[f, 0]])) * 1e6), 'stored': float(0.5 * np.linalg.norm(np.cross(X_stored[T[f, 1]] - X_stored[T[f, 0]], X_stored[T[f, 2]] - X_stored[T[f, 0]])) * 1e6)}}
    if X_pre is not None:
        npre = FQ.face_normals(X_pre, T[[f]])[0]; cp = X_pre[T[f]].mean(0); odp = (cp - c_cap) / max(np.linalg.norm(cp - c_cap), 1e-12); out['radial_dot_earlier_phase'] = float(npre @ odp); out['earlier_phase'] = pre_phase; out['max_move_earlier_to_stored_mm'] = float(np.abs(X_pre[T[f]] - X_stored[T[f]]).max() * 1000)
        out['storage_attribution'] = ('storage_created' if out['radial_dot_earlier_phase'] >= 0 > out['radial_dot_stored'] else 'present_before_storage') if pre_phase == 'pre_quantization_continuous' else 'unknown_(no pre-lattice snapshot preserved)'
    else: out['radial_dot_earlier_phase'] = None; out['earlier_phase'] = 'none'; out['storage_attribution'] = 'unknown_(no earlier geometry)'
    sv, sgn = face_jacobian(P[T[f]], X_stored[T[f]], nd); out['stretch_singular_values_vs_donor'] = [float(x) for x in sv]; out['donor_facing_sign'] = sgn; out['donor_facing_note'] = 'sign(n_stored . n_donor): a donor-facing comparison, flipped by a proper 180-degree rigid rotation too; NOT an intrinsic inversion proof'
    nb = adj_faces.get(int(f), []); unit = lambda m: m / max(np.linalg.norm(m), 1e-12)
    out['neighbour_continuity_deg'] = {'stored': float(np.degrees(np.arccos(np.clip(ns @ unit(FQ.face_normals(X_stored, T[nb]).sum(0)), -1, 1)))) if nb else None, 'donor': float(np.degrees(np.arccos(np.clip(nd @ unit(FQ.face_normals(P, T[nb]).sum(0)), -1, 1)))) if nb else None}
    ed = np.flatnonzero((ES['f'] == f) | (ES['g'] == f)) if ES['n'] else np.zeros(0, int); th_s = CR.signed_dihedrals(X_stored * 1000, T, ES) if ES['n'] else np.zeros(0)
    out['edges'] = [{'edge': [int(ES['a'][i]), int(ES['b'][i])], 'donor_deg': float(np.degrees(th_d[i])), 'stored_deg': float(np.degrees(th_s[i])), 'crease_edge': bool(is_crease[i]), 'sign_flip': bool(is_crease[i] and np.sign(th_s[i]) != np.sign(th_d[i])), 'outside_envelope_deg': float(np.degrees(max(0, th_s[i] - env_hi[i]) + min(0, th_s[i] - env_lo[i])))} for i in ed]
    out['in_fold_pair'] = int(f) in fold_faces; out['in_self_intersection'] = int(f) in selfx_faces; out['inverted_local_(vs adjacent faces)'] = int(f) in inverted_local_faces
    structural = out['in_fold_pair'] or out['in_self_intersection'] or out['inverted_local_(vs adjacent faces)'] or any(e['sign_flip'] for e in out['edges'])
    disc = out['neighbour_continuity_deg']; discont = disc['stored'] is not None and disc['stored'] > disc['donor'] + 60
    if structural or discont: out['disposition'] = 'confirmed_structural'
    elif out['storage_attribution'] == 'storage_created': out['disposition'] = 'storage_created_orientation'
    elif out['donor_normal_dot'] >= 0.7 and all(abs(e['outside_envelope_deg']) < 1e-9 for e in out['edges']) and out['radial_dot_stored'] > -0.35: out['disposition'] = 'anatomical_roll_candidate_(provisional)'
    else: out['disposition'] = 'ambiguous_(blocking)'
    return out


def coordinate_roundtrip_repeat(X, T, ES):
    """Float32 coordinate round trip and repeat execution of the dihedral formula on identical input (deg). This is NOT a total angular
    error measurement and NOT a permuted evaluation."""
    th = CR.signed_dihedrals(X, T, ES); th32 = CR.signed_dihedrals(X.astype(np.float32).astype(np.float64), T, ES); th_b = CR.signed_dihedrals(np.array(X, copy=True), T, ES)
    d32 = np.degrees(np.abs(np.arctan2(np.sin(th32 - th), np.cos(th32 - th)))); return {'float32_coordinate_roundtrip_max_deg': float(d32.max()) if len(d32) else 0.0, 'repeat_execution_max_deg': float(np.degrees(np.abs(th_b - th)).max()) if len(th) else 0.0, 'note': 'coordinate round trip + repeatability only; not total angular error'}
