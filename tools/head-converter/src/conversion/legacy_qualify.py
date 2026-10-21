"""Static qualification measurements for Legacy donor-fit candidates (numpy + Pillow only). Every measurement is
applied identically to the untouched native donor, the CC5 source where meaningful, and a candidate, so numbers are
read as differences from the donor rather than absolutes. Nothing here grants readiness: `readiness_report` derives
a conservative, machine-readable status from the measured evidence and always keeps `export_ready: false` and
`ready_for_experimental_packing: false` until Codex's native evidence exists.

Contents
  ortho_raycast / visibility     binned orthographic ray cast (same views as render_legacy.py): first-hit component and
                                 facing per pixel; back-face-first pixels = interior exposed through an opening
  cap_contact                    lid/socket records vs an eye cap: sphere approximation, cap angular extent and the
                                 actual cap triangles (signed), plus exact triangle-triangle intersections
  haircap_relation               head records vs the haircap shell: covered / open / poking-through by ray tests,
                                 signed clearance, exact intersections
  uv_template_evidence           twin residuals by region, UV-edge agreement and seam agreement between two per-record
                                 UV layouts (chart topology evidence, not just nearest twins)
  readiness_report               `claude-legacy-readiness/1`
Frame: Blender (Z up, -Y forward) everywhere; callers convert native positions with geometry.native_to_blender.
"""
import numpy as np
from . import geometry as G, face_quality as FQ

READINESS_SCHEMA = 'claude-legacy-readiness/1'
VIEWS = {'front': (0, -1, 0), 'left_profile': (1, 0, 0), 'right_profile': (-1, 0, 0), 'rear': (0, 1, 0), 'under_jaw': (0, -1, -0.8), 'top': (0, 0, 1)}
FORWARD = np.array([0.0, -1.0, 0.0])


# ---- orthographic ray cast --------------------------------------------------------------------------------------
def view_axes(d):
    d = np.asarray(d, float); d = d / np.linalg.norm(d)
    up = np.array([0.0, 0.0, 1.0]) if abs(d[2]) < 0.9 else np.array([0.0, 1.0, 0.0])
    x = np.cross(up, d); x /= np.linalg.norm(x); y = np.cross(d, x); return d, x, y


def ortho_raycast(P, T, d, res=(240, 270), scale=0.46, focus=(0.0, 0.0, 1.75), max_pairs=40_000_000):
    """Orthographic camera at focus + d looking along -d (render_legacy.py convention). Returns (face map HxW, -1 =
    background; depth map; front-facing bool per face = winding normal towards the camera). Vectorised: every
    triangle is tested only against the pixels of its screen bounding box."""
    d, xa, ya = view_axes(d); W, H = res; P = np.asarray(P, float); T = np.asarray(T, int); Q = P - np.asarray(focus, float)
    px = (Q @ xa / scale + 0.5) * W; py = (0.5 - Q @ ya / scale) * H; depth = -(Q @ d)
    X = px[T]; Y = py[T]; Z = depth[T]
    x0 = np.clip(np.ceil(X.min(1) - 0.5), 0, W - 1).astype(int); x1 = np.clip(np.floor(X.max(1) - 0.5), -1, W - 1).astype(int)
    y0 = np.clip(np.ceil(Y.min(1) - 0.5), 0, H - 1).astype(int); y1 = np.clip(np.floor(Y.max(1) - 0.5), -1, H - 1).astype(int)
    w = np.maximum(x1 - x0 + 1, 0); h = np.maximum(y1 - y0 + 1, 0); cnt = w * h; valid = cnt > 0
    fn = np.cross(P[T[:, 1]] - P[T[:, 0]], P[T[:, 2]] - P[T[:, 0]]); front = (fn @ d) > 0
    face = np.full((H, W), -1, int); dmap = np.full((H, W), np.inf)
    if not valid.any(): return face, dmap, front
    tri = np.flatnonzero(valid); c = cnt[tri]
    if c.sum() > max_pairs: raise ValueError(f'ray cast would test {int(c.sum())} pixel-triangle pairs; lower the resolution')
    rep = np.repeat(tri, c); start = np.repeat(np.cumsum(c) - c, c); local = np.arange(len(rep)) - start
    ix = x0[rep] + local % w[rep]; iy = y0[rep] + local // w[rep]; cx = ix + 0.5; cy = iy + 0.5
    xa0, xa1, xa2 = X[rep, 0], X[rep, 1], X[rep, 2]; ya0, ya1, ya2 = Y[rep, 0], Y[rep, 1], Y[rep, 2]
    det = (xa1 - xa0) * (ya2 - ya0) - (xa2 - xa0) * (ya1 - ya0); ok = np.abs(det) > 1e-14; det = np.where(ok, det, 1.0)
    l1 = ((cx - xa0) * (ya2 - ya0) - (xa2 - xa0) * (cy - ya0)) / det; l2 = ((xa1 - xa0) * (cy - ya0) - (cx - xa0) * (ya1 - ya0)) / det; l0 = 1 - l1 - l2
    inside = ok & (l0 >= -1e-9) & (l1 >= -1e-9) & (l2 >= -1e-9)
    if not inside.any(): return face, dmap, front
    rep, ix, iy, l0, l1, l2 = rep[inside], ix[inside], iy[inside], l0[inside], l1[inside], l2[inside]
    z = l0 * Z[rep, 0] + l1 * Z[rep, 1] + l2 * Z[rep, 2]; key = iy * W + ix
    order = np.lexsort((z, key)); key_s = key[order]; first = np.r_[True, key_s[1:] != key_s[:-1]]; sel = order[first]
    face[iy[sel], ix[sel]] = rep[sel]; dmap[iy[sel], ix[sel]] = z[sel]; return face, dmap, front


def visibility(components, view, res=(240, 270), scale=0.46, focus=(0.0, 0.0, 1.75)):
    """components: ordered dict name -> (positions, triangles) in the Blender frame. Returns (class map HxW of
    strings, counts). Classes: 'background', '<name>:front', '<name>:back' (a back face seen first = looking at the
    inside of that shell through an opening)."""
    names = list(components); Ps = []; Ts = []; labels = []; off = 0
    for n in names:
        P, T = components[n]; P = np.asarray(P, float); T = np.asarray(T, int); Ps.append(P); Ts.append(T + off); labels.append(np.full(len(T), names.index(n))); off += len(P)
    P = np.vstack(Ps); T = np.vstack(Ts); lab = np.concatenate(labels); face, dmap, front = ortho_raycast(P, T, VIEWS[view] if isinstance(view, str) else view, res, scale, focus)
    cls = np.full(face.shape, 'background', object); hit = face >= 0
    cls[hit] = np.where(front[face[hit]], np.array([n + ':front' for n in names], object)[lab[face[hit]]], np.array([n + ':back' for n in names], object)[lab[face[hit]]])
    u, c = np.unique(cls, return_counts=True); return cls, {str(k): int(v) for k, v in zip(u, c)}


PALETTE = {'background': (255, 255, 255), 'head:front': (200, 160, 120), 'head:back': (255, 0, 200), 'haircap:front': (70, 55, 45), 'haircap:back': (255, 140, 0), 'eye_r:front': (170, 200, 255), 'eye_l:front': (170, 200, 255),
           'eye_r:back': (255, 0, 0), 'eye_l:back': (255, 0, 0), 'mouthbag:front': (130, 40, 40), 'mouthbag:back': (150, 0, 255), 'source:front': (140, 160, 190), 'source:back': (0, 200, 255)}


def class_image(cls, path, upscale=3):
    """Write the class map as a PNG (Pillow); colours from PALETTE, unknown classes grey."""
    from PIL import Image
    H, W = cls.shape; img = np.zeros((H, W, 3), np.uint8)
    for k in np.unique(cls): img[cls == k] = PALETTE.get(str(k), (128, 128, 128))
    Image.fromarray(np.repeat(np.repeat(img, upscale, 0), upscale, 1)).save(path)


# ---- exact cross-mesh intersections --------------------------------------------------------------------------------
def cross_intersections(P1, T1, P2, T2, faces1=None, chunk=512):
    """Exact triangle-triangle intersections between two meshes (bbox pruned). Returns sorted (i, j) pairs."""
    P1 = np.asarray(P1, float); T1 = np.asarray(T1, int); P2 = np.asarray(P2, float); T2 = np.asarray(T2, int)
    lo1 = P1[T1].min(1); hi1 = P1[T1].max(1); lo2 = P2[T2].min(1); hi2 = P2[T2].max(1)
    cand = np.arange(len(T1)) if faces1 is None else np.asarray(faces1, int); pairs = []
    for s in range(0, len(cand), chunk):
        ci = cand[s:s + chunk]; ov = (lo1[ci][:, None, :] <= hi2[None, :, :]).all(2) & (hi1[ci][:, None, :] >= lo2[None, :, :]).all(2)
        for a, b in zip(*np.nonzero(ov)):
            i, j = int(ci[a]), int(b)
            if FQ._tri_tri_intersect(P1[T1[i, 0]], P1[T1[i, 1]], P1[T1[i, 2]], P2[T2[j, 0]], P2[T2[j, 1]], P2[T2[j, 2]]): pairs.append((i, j))
    return sorted(pairs)


def _stats(d):
    d = np.asarray(d, float)
    return {'n': int(len(d)), 'min_mm': float(d.min() * 1000), 'median_mm': float(np.median(d) * 1000), 'max_mm': float(d.max() * 1000)} if len(d) else {'n': 0}


# ---- eye cap vs lids ------------------------------------------------------------------------------------------------
def cap_contact(head_P, head_T, cap_P, cap_T, lid_ids, head_faces=None, sphere=None, forward=None):
    """Lid records vs one eye cap with three comparable readings: signed distance to the fitted sphere, whether the
    record lies inside the cap's angular extent (outside it the sphere continues but no cap geometry exists), and
    signed distance to the actual cap triangles (positive = farther from the cap centre than the nearest cap point).
    Adds exact intersections between the given head faces and the cap, and the cap records' position relative to
    the head skin (outside = visible). `sphere` = (centre, radius) overrides the fitted sphere for the sphere-only
    reading (used to reproduce the CC5-radius gate on the same records)."""
    from .eye_fit import sphere_fit
    head_P = np.asarray(head_P, float); head_T = np.asarray(head_T, int); cap_P = np.asarray(cap_P, float); cap_T = np.asarray(cap_T, int); lid_ids = np.asarray(lid_ids, int)
    c, r, res = sphere_fit(cap_P); cs, rs = (c, r) if sphere is None else (np.asarray(sphere[0], float), float(sphere[1]))
    u = (cap_P - c) / np.linalg.norm(cap_P - c, axis=1)[:, None]; ax = u.mean(0) if forward is None else np.asarray(forward, float); ax = ax / np.linalg.norm(ax)
    half = float(np.degrees(np.arccos(np.clip(u @ ax, -1, 1))).max())
    v = head_P[lid_ids] - cs; dist = np.linalg.norm(v, axis=1); sd_sphere = dist - rs
    ang = np.degrees(np.arccos(np.clip(((head_P[lid_ids] - c) / np.maximum(np.linalg.norm(head_P[lid_ids] - c, axis=1), 1e-12)[:, None]) @ ax, -1, 1))); within = ang <= half
    soup = G.TriangleSoup(cap_P, cap_T); sd_cap = np.zeros(len(lid_ids))
    for k, i in enumerate(lid_ids):
        q, dd = soup.nearest(head_P[i]); sd_cap[k] = dd * (1 if np.linalg.norm(head_P[i] - c) >= np.linalg.norm(q - c) else -1)
    inside_sphere = sd_sphere < 0; inside_cap = (sd_cap < 0) & within
    faces = np.arange(len(head_T)) if head_faces is None else np.asarray(head_faces, int)
    near = faces[(np.linalg.norm(head_P[head_T[faces]].mean(1) - c, axis=1) < r + 0.02)] if len(faces) else faces; xs = cross_intersections(head_P, head_T, cap_P, cap_T, near) if len(near) else []
    cap_out = np.zeros(len(cap_P))
    if len(faces):     # cap records relative to the head skin (positive = outside the skin = visible)
        hs = G.TriangleSoup(head_P, head_T[faces]); hfn = FQ.face_normals(head_P, head_T[faces])
        for k in range(len(cap_P)):
            q, dd, f = hs.nearest_face(cap_P[k]); cap_out[k] = dd * (1 if (cap_P[k] - q) @ hfn[f] >= 0 else -1)
    return {'sphere': {'centre': c.tolist(), 'radius_mm': float(r * 1000), 'rms_mm': float(np.sqrt((res ** 2).mean()) * 1000), 'half_angle_deg': half, 'forward_axis': ax.tolist(), 'reading_sphere': {'centre': cs.tolist(), 'radius_mm': float(rs * 1000)}},
            'lid_records': int(len(lid_ids)), 'inside_sphere': int(inside_sphere.sum()), 'inside_sphere_within_cap_extent': int((inside_sphere & within).sum()), 'inside_sphere_outside_cap_extent': int((inside_sphere & ~within).sum()),
            'inside_actual_cap_surface': int(inside_cap.sum()), 'min_signed_to_sphere_mm': float(sd_sphere.min() * 1000) if len(lid_ids) else None, 'min_signed_to_cap_surface_within_extent_mm': float(sd_cap[within].min() * 1000) if within.any() else None,
            'min_signed_to_cap_surface_any_mm': float(sd_cap.min() * 1000) if len(lid_ids) else None, 'penetrating_ids': lid_ids[inside_cap].tolist(), 'penetrating_depth_mm': (-sd_cap[inside_cap] * 1000).round(3).tolist(),
            'head_cap_triangle_intersections': len(xs), 'head_faces_tested': int(len(near)), 'cap_records_outside_skin': int((cap_out > 0).sum()), 'cap_records_total': int(len(cap_P)), 'cap_apex_outside_skin_mm': float(cap_out.max() * 1000), 'cap_rim_depth_under_skin_mm': float(-cap_out.min() * 1000)}


def contact_verdict(donor, candidate, source_reading=None):
    """Classify a candidate's cap/lid reading relative to the donor's (same method): approximation artifact,
    contact within the donor's own envelope, or a new defect. `source_reading` = the same records vs the CC5-radius
    sphere (what the run-04 gate measured) makes the radius-policy component explicit."""
    n_sphere = candidate['inside_sphere']; n_extent = candidate['inside_sphere_within_cap_extent']; n_cap = candidate['inside_actual_cap_surface']; xs = candidate['head_cap_triangle_intersections']
    depth = max((candidate.get('penetrating_depth_mm') or [0.0]), default=0.0); donor_margin = donor['min_signed_to_cap_surface_within_extent_mm']
    # face intersections exist on the donor too (the lid margin tucks under the cap rim), so they are read relative to the
    # donor; the record readings decide the kind
    if n_sphere == 0 and n_cap == 0: kind = 'no_penetration'
    elif n_extent == 0 and n_cap == 0: kind = 'approximation_artifact'                        # inside the sphere only where no cap geometry exists
    elif source_reading is not None and (source_reading['min_signed_to_sphere_mm'] or 0) < 0: kind = 'fit_defect'   # inside even the CC5-radius sphere: the registration pushed the lid into the eyeball
    else: kind = 'surface_contact'                                                              # lid records under the actual cap surface within its extent
    out = {'kind': kind, 'max_depth_mm': float(depth), 'donor_min_clearance_mm': donor_margin, 'candidate_min_clearance_mm': candidate['min_signed_to_cap_surface_within_extent_mm'], 'donor_intersections': donor['head_cap_triangle_intersections'], 'candidate_intersections': xs,
           'intersections_read': 'relative to the donor (its lid margin already crosses the cap rim); not a kind criterion'}
    if source_reading is not None:
        out['same_records_vs_cc5_radius_sphere_min_mm'] = source_reading['min_signed_to_sphere_mm']; out['radius_delta_mm'] = candidate['sphere']['radius_mm'] - source_reading['sphere']['reading_sphere']['radius_mm']
        out['radius_policy_component'] = 'the CC5-radius reading is non-negative on the same records: the penetration equals the cap radius surplus consumed, a declared eye-radius policy consequence, not a registration defect' if (source_reading['min_signed_to_sphere_mm'] or 0) >= 0 else 'records are also inside the CC5-radius sphere: a fit defect independent of the radius policy'
    return out


def socket_contact(ex_donor, ex_cand, mk, rings=2, tol_mm=1.0):
    """Eye-socket rim (atlas lash-band records: the lid margin resting on the donor eyeball + the socket-fan apex) and
    the `rings` rows of outer-shell records around it, read per side against the PLACED cap with `cap_contact` (any
    palette; the eyelid-bucket screen never covered player_67's eye_control-weighted upper-margin rows). Also the
    rim's donor relation: |signed distance to the placed cap - signed distance of the donor record to the donor cap|,
    the quantity the socket_rim_policy preserves. Returns per side readings plus a summary usable as a screen."""
    from . import legacy_exchange as LX, legacy_fit as LF
    P, T = mk['P'], mk['T']; hp = G.native_to_blender(LX.submesh(ex_cand, 'head')['positions']); outer = np.flatnonzero(mk['outer_faces']); lash = mk['lash']
    sets = LF.socket_rim_sets(ex_donor, mk, tol_mm); out = {'rings': rings, 'tol_mm': tol_mm, 'sides': {}, 'unbanded_on_cap': sets['unbanded_on_cap']}
    for side, role in (('r', 'eye_r'), ('l', 'eye_l')):
        sd_ = sets['sides'][side]; rim = np.array(sd_['ids'], int); c0 = np.array(sd_['centre']); r0 = sd_['radius_m']
        ring = np.zeros(len(P), bool)
        if len(rim): ring[G.grow(rim, mk['adj'], rings)] = True
        ring_ids = np.flatnonzero(ring & mk['on_skin'] & ~lash & ~mk['is_frozen'])
        cp = G.native_to_blender(LX.submesh(ex_cand, role)['positions']); ct = np.array(LX.submesh(ex_cand, role)['triangles'])
        from .eye_fit import sphere_fit
        c1, r1, _ = sphere_fit(cp)
        rim_read = cap_contact(hp, T, cp, ct, rim, head_faces=outer) if len(rim) else None; ring_read = cap_contact(hp, T, cp, ct, ring_ids, head_faces=outer) if len(ring_ids) else None
        sd_cand = (np.linalg.norm(hp[rim] - c1, axis=1) - r1) * 1000 if len(rim) else np.zeros(0); sd_don = np.array([sd_['donor_signed_mm'][int(i)] for i in rim]) if len(rim) else np.zeros(0)
        dev = np.abs(sd_cand - sd_don)
        out['sides'][side] = {'role': role, 'character_side': sd_['character_side'], 'rim_ids': rim.tolist(), 'rim_on_cap': sd_['on_cap'], 'rim_interior': sd_['interior'], 'ring_ids': ring_ids.tolist(),
                              'rim': {k: rim_read[k] for k in ('inside_sphere', 'inside_sphere_within_cap_extent', 'inside_actual_cap_surface', 'min_signed_to_sphere_mm', 'min_signed_to_cap_surface_within_extent_mm', 'penetrating_ids', 'penetrating_depth_mm', 'head_cap_triangle_intersections')} if rim_read else None,
                              'ring': {k: ring_read[k] for k in ('lid_records', 'inside_sphere', 'inside_sphere_within_cap_extent', 'inside_actual_cap_surface', 'min_signed_to_sphere_mm', 'min_signed_to_cap_surface_within_extent_mm', 'penetrating_ids', 'penetrating_depth_mm')} if ring_read else None,
                              'rim_signed_to_placed_sphere_mm': {int(i): float(v) for i, v in zip(rim, sd_cand)}, 'rim_donor_relation_deviation_mm': {'max': float(dev.max()) if len(dev) else 0.0, 'median': float(np.median(dev)) if len(dev) else 0.0},
                              'on_cap_rim_inside_placed_sphere_beyond_tol': int((sd_cand[np.isin(rim, sd_['on_cap'])] < -tol_mm).sum()) if len(rim) else 0}
    out['summary'] = {'rim_records_inside_actual_cap': {s: (v['rim']['inside_actual_cap_surface'] if v['rim'] else 0) for s, v in out['sides'].items()}, 'ring_records_inside_actual_cap': {s: (v['ring']['inside_actual_cap_surface'] if v['ring'] else 0) for s, v in out['sides'].items()},
                      'on_cap_rim_inside_beyond_tol': {s: v['on_cap_rim_inside_placed_sphere_beyond_tol'] for s, v in out['sides'].items()}, 'rim_donor_relation_max_dev_mm': {s: v['rim_donor_relation_deviation_mm']['max'] for s, v in out['sides'].items()}}
    return out


# ---- haircap vs head --------------------------------------------------------------------------------------------------
def haircap_relation(head_P, head_T, cap_P, cap_T, ids, merge_groups=(), head_faces=None, max_gap_m=0.03, poke_m=0.005):
    """For head records `ids`: signed clearance to the haircap shell (positive = outside its outward face), and
    travel-limited ray tests along the record's outward normal: covered (cap above within max_gap), poking (cap within
    poke_m below and nothing above = skin through the hair shell), open (neither; a far hit from inside the skull does
    not count). Exact head/cap intersections on `head_faces`."""
    head_P = np.asarray(head_P, float); head_T = np.asarray(head_T, int); cap_P = np.asarray(cap_P, float); cap_T = np.asarray(cap_T, int); ids = np.asarray(ids, int)
    vn = G.vertex_normals(head_P, head_T, merge_groups); soup = G.TriangleSoup(cap_P, cap_T); cfn = FQ.face_normals(cap_P, cap_T)
    sd = np.zeros(len(ids)); state = np.empty(len(ids), object)
    for k, i in enumerate(ids):
        p = head_P[i]; q, dd, f = soup.nearest_face(p); sd[k] = dd * (1 if (p - q) @ cfn[f] >= 0 else -1)
        above = soup.ray_hit(p, vn[i]); below = soup.ray_hit(p, -vn[i])
        state[k] = 'covered' if (above is not None and above < max_gap_m) else ('poking' if (below is not None and below < poke_m) else 'open')
    faces = np.arange(len(head_T)) if head_faces is None else np.asarray(head_faces, int)
    lo, hi = cap_P.min(0) - 0.002, cap_P.max(0) + 0.002; near = faces[((head_P[head_T[faces]] >= lo) & (head_P[head_T[faces]] <= hi)).all(2).any(1)]
    xs = cross_intersections(head_P, head_T, cap_P, cap_T, near)
    under = state != 'open'; pk = state == 'poking'
    return {'records': int(len(ids)), 'clearance_mm': _stats(np.abs(sd)), 'signed_clearance_mm': {'min': float(sd.min() * 1000), 'median': float(np.median(sd) * 1000), 'max': float(sd.max() * 1000)} if len(ids) else {},
            'covered': int((state == 'covered').sum()), 'poking': int(pk.sum()), 'open': int((state == 'open').sum()), 'poking_ids': ids[pk].tolist(), 'open_ids': ids[state == 'open'].tolist(),
            'under_cap_signed_clearance_mm': {'median': float(np.median(sd[under]) * 1000), 'p05': float(np.percentile(sd[under], 5) * 1000), 'p95': float(np.percentile(sd[under], 95) * 1000)} if under.any() else {},
            'under_cap_coincident_within_0p5mm': int((under & (np.abs(sd) < 0.0005)).sum()), 'poking_above_0p5mm': int((pk & (sd > 0.0005)).sum()), 'poking_above_2mm': int((pk & (sd > 0.002)).sum()), 'under_cap_below_minus_2mm': int((under & (sd < -0.002)).sum()), 'under_cap_below_minus_5mm': int((under & (sd < -0.005)).sum()),
            'head_cap_triangle_intersections': len(xs), 'intersecting_head_faces': sorted({i for i, _ in xs})[:200], 'head_faces_tested': int(len(near)),
            'reading': 'covered = cap directly above within 30 mm (player_67: hair volume 0-8 mm over the scalp, rim on the skin); poking = cap within 5 mm below the record and nothing above = skin outside the hair shell; below-thresholds = gap under the cap'}


# ---- UV template evidence -------------------------------------------------------------------------------------------
def _edges(T):
    T = np.asarray(T, int); E = np.vstack([T[:, [0, 1]], T[:, [1, 2]], T[:, [2, 0]]]); E.sort(1); return np.unique(E, axis=0)


def uv_template_evidence(U_a, T_a, U_b, T_b, regions_a=None, seam_groups_a=(), seam_groups_b=(), px=1024):
    """Layout A (e.g. Legacy per-record UVs) against layout B (e.g. atlas-A Ekblad): nearest twins with residuals by
    region, fraction of A's UV edges whose twin pair is also an edge of B (chart connectivity, not just point
    proximity), and seam agreement: A's seam groups (records sharing a position with different UVs) whose twins are
    also members of one B seam group. Brute force; fine for a few thousand records."""
    U_a = np.asarray(U_a, float); U_b = np.asarray(U_b, float)
    twin = np.zeros(len(U_a), int); res = np.zeros(len(U_a))
    for s in range(0, len(U_a), 256):
        d = np.linalg.norm(U_a[s:s + 256, None, :] - U_b[None, :, :], axis=2); twin[s:s + 256] = d.argmin(1); res[s:s + 256] = d.min(1)
    res_px = res * px; out = {'records_a': int(len(U_a)), 'records_b': int(len(U_b)), 'distinct_twins': int(len(np.unique(twin))), 'twin_residual_px': {'median': float(np.median(res_px)), 'p95': float(np.percentile(res_px, 95)), 'max': float(res_px.max())}, 'px': px}
    if regions_a:
        out['by_region_px'] = {}
        for name, m in regions_a.items():
            m = np.asarray(m, bool)
            if m.any(): out['by_region_px'][name] = {'n': int(m.sum()), 'median': float(np.median(res_px[m])), 'p95': float(np.percentile(res_px[m], 95)), 'max': float(res_px[m].max())}
    Ea = _edges(T_a); Eb = _edges(T_b); eb = {(int(a), int(b)) for a, b in Eb}
    tw = np.sort(twin[Ea], axis=1); agree = np.array([(int(a), int(b)) in eb for a, b in tw]); degenerate = tw[:, 0] == tw[:, 1]
    nb = {}
    for a, b in Eb: nb.setdefault(int(a), set()).add(int(b)); nb.setdefault(int(b), set()).add(int(a))
    two_hop = np.array([bool(nb.get(int(a), set()) & nb.get(int(b), set())) for a, b in tw])     # twins share a B neighbour: A skips one B record on the same chart
    out['uv_edges_a'] = int(len(Ea)); out['uv_edges_with_twin_edge_in_b'] = int(agree.sum()); out['uv_edge_agreement'] = float(agree.mean()); out['edges_collapsing_to_one_twin'] = int(degenerate.sum())
    out['uv_edges_with_twin_edge_or_shared_b_neighbour'] = int((agree | two_hop).sum()); out['uv_edge_agreement_within_2_hops'] = float((agree | two_hop).mean())
    gb = {}
    for gi, ids in enumerate(seam_groups_b):
        for i in ids: gb[int(i)] = gi
    seam_ok = 0; seam_split = 0; seam_missing = 0
    for ids in seam_groups_a:
        gs = {gb.get(int(twin[i]), None) for i in ids}
        if None in gs: seam_missing += 1
        elif len(gs) == 1: seam_ok += 1
        else: seam_split += 1
    out['seams_a'] = int(len(seam_groups_a)); out['seams_b'] = int(len(seam_groups_b)); out['seams_a_mapping_into_one_b_seam'] = seam_ok; out['seams_a_split_across_b_seams'] = seam_split; out['seams_a_with_non_seam_twins'] = seam_missing
    out['twins'] = twin.tolist(); return out


# ---- readiness --------------------------------------------------------------------------------------------------------
SCREENING_GATES = {
    'attachment_exact': {'screens': 'frozen attachment loop + rings byte-equal to the donor', 'does_not_prove': 'body join at runtime (jersey/neck fixture absent)'},
    'seams_coincident': {'screens': 'seam-group records share one position after the fit', 'does_not_prove': 'normal/tangent continuity (writer regenerates)'},
    'no_new_collapsed_triangles': {'screens': 'no triangle collapsed to zero area by the fit', 'does_not_prove': 'rendering quality of slivers below the sliver threshold'},
    'inverted_not_worse_than_donor_plus_10_excluding_lash': {'screens': 'locally inverted faces (lash strips excluded) within donor + 10', 'does_not_prove': 'that the remaining inverted faces are invisible in game'},
    'no_eyelid_records_inside_cc5_eye': {'screens': 'outer-shell eyelid-bucket records outside the CC5-radius sphere at the placed centre', 'does_not_prove': 'clearance against the 16 mm Legacy cap actually placed (measured separately by cap_contact)'},
    'mouth_loop_not_collapsed': {'screens': 'mouth-loop height above 50 % of the donor rest gap', 'does_not_prove': 'that the rest gap is what the runtime expects (parted-lip policy question)'},
    'no_lid_record_inside_placed_cap_within_extent': {'screens': 'no outer-shell lid record inside the actual placed cap surface within the cap angular extent', 'does_not_prove': 'lid/cap behaviour during blink (eyelid bones move the lids, caps are head-rigid)'}}


def readiness_report(baseline, candidates, acceptance, statically_qualified, unknowns, failure_criteria, pack_first, codex_dependencies, notes=()):
    """Conservative machine-readable readiness. `candidates`: list of dicts with id, path, sha256, differs_from_baseline,
    screening_gates {name: bool}, gate_waivers [...]. `acceptance`: unresolved acceptance requirements with state in
    {'measured_only', 'unknown', 'blocked'}. export_ready / ready_for_experimental_packing are always False here: only
    Codex's native reparse + a game observation can change them, and that must be a new record, not an edit."""
    for c in candidates:
        c['all_screening_gates_pass'] = bool(all(c['screening_gates'].values())); c['status'] = 'experimental_not_for_packing'
    return {'schema': READINESS_SCHEMA, 'export_ready': False, 'ready_for_experimental_packing': False, 'packing_owner': 'Codex (positions-only Legacy repack + reparse receipt); this report never authorises a write',
            'baseline': baseline, 'candidates': candidates, 'screening_gates': SCREENING_GATES, 'acceptance_requirements_unresolved': acceptance, 'statically_qualified': statically_qualified, 'unknown': unknowns,
            'failure_criteria': failure_criteria, 'pack_first': pack_first, 'codex_dependencies': codex_dependencies, 'notes': list(notes),
            'reading_rule': 'screening gates are conversion-side structural screens on copies; acceptance requirements need native reparse, decoded texture or game observation; a passing screen never implies acceptance'}


# ---- pinned inputs, quantisation, screen set ---------------------------------------------------------------------------
def verify_pins(handoff):
    """Every {path, sha256} entry of a handoff (top level and implementation_provenance) re-hashed. Returns
    (inputs_ok, rows [{key, path, ok, sha256_expected, sha256_actual, kind input|provenance}]). Inputs must verify;
    provenance rows (implementation files) are reported, and a caller must state any drift and its cause."""
    import hashlib
    from pathlib import Path
    rows = []
    items = [(k, v) for k, v in handoff.items() if isinstance(v, dict) and 'path' in v and 'sha256' in v]
    items += [('implementation_provenance', v) for v in handoff.get('implementation_provenance', [])]
    for k, v in items:
        p = Path(v['path']); actual = hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else None
        rows.append({'key': k, 'path': str(p), 'ok': actual == v['sha256'], 'sha256_expected': v['sha256'], 'sha256_actual': actual, 'kind': 'provenance' if k == 'implementation_provenance' else 'input'})
    return all(r['ok'] for r in rows if r['kind'] == 'input'), rows


def quantization_delta(ex_a, ex_b, roles=('head', 'eye_r', 'eye_l', 'mouthbag', 'haircap')):
    """Per-role position difference b - a (native frame, mm): max, rms, count above 0.25 / 0.5 mm."""
    from . import legacy_exchange as LX
    out = {}
    for r in roles:
        A = np.asarray(LX.submesh(ex_a, r)['positions'], float); B = np.asarray(LX.submesh(ex_b, r)['positions'], float); d = np.linalg.norm(B - A, axis=1) * 1000
        out[r] = {'records': int(len(d)), 'max_mm': float(d.max()), 'rms_mm': float(np.sqrt((d ** 2).mean())), 'over_0p25mm': int((d > 0.25).sum()), 'over_0p5mm': int((d > 0.5).sum())}
    return out


def screen_set(ex_donor, ex_cand, mk, res=(200, 200), lid_faces_self=True):
    """The conversion-side screens on one assembled Legacy exchange (positions may be pre- or post-quantisation),
    read against the untouched donor with the same method: attachment max error, seam max gap, collapsed and
    inverted faces (lash excluded), placed-cap contact per eye, mouth slit (front zoom), mouth-loop height ratio,
    haircap covered/poking/open, mouthbag/head intersections, self-intersections among lid+lip faces."""
    from . import legacy_exchange as LX, face_quality as FQ
    P, T, U = mk['P'], mk['T'], mk['U']; hp = G.native_to_blender(LX.submesh(ex_cand, 'head')['positions']); outer = np.flatnonzero(mk['outer_faces']); lash = mk['lash']; dup = mk['dup']
    frozen = mk['frozen']; att = float(np.abs(hp[frozen] - P[frozen]).max()); seam = max((float(np.linalg.norm(hp[ids] - hp[ids[0]], axis=1).max()) for ids in dup), default=0.0)
    q = G.triangle_quality(P, hp, T); mf = FQ.face_metrics(hp, T, dup); md = FQ.face_metrics(P, T, dup); nolash = ~lash[T].any(1)
    inv_fit = int((mf['inverted_local'] & nolash).sum()); inv_donor = int((md['inverted_local'] & nolash).sum())
    eyes = {}
    for s, role in (('r', 'eye_r'), ('l', 'eye_l')):
        lid = np.flatnonzero((mk['bucket'][f'upper_eyelid_{s}'] | mk['bucket'][f'lower_eyelid_{s}']) & mk['on_skin'] & ~lash); cp = G.native_to_blender(LX.submesh(ex_cand, role)['positions']); ct = np.array(LX.submesh(ex_cand, role)['triangles'])
        r = cap_contact(hp, T, cp, ct, lid, head_faces=outer); r.pop('penetrating_ids', None); eyes[role] = r
    ml = mk['mouth_loop']; comps = {r: (G.native_to_blender(LX.submesh(ex_cand, r)['positions']), np.array(LX.submesh(ex_cand, r)['triangles'])) for r in ('head', 'eye_r', 'eye_l', 'mouthbag', 'haircap')}
    cls, cnt = visibility(comps, 'front', res=res, scale=0.08, focus=hp[ml].mean(0)); slit = int(sum(v for k, v in cnt.items() if k != 'head:front'))
    h_don = float((P[ml, 2].max() - P[ml, 2].min())); h_fit = float((hp[ml, 2].max() - hp[ml, 2].min()))
    hair = haircap_relation(hp, T, comps['haircap'][0], comps['haircap'][1], np.arange(len(hp)), dup, head_faces=outer)
    for k in ('poking_ids', 'open_ids', 'intersecting_head_faces'): hair.pop(k, None)
    mp, mt = comps['mouthbag']; near = outer[np.linalg.norm(hp[T[outer]].mean(1) - hp[ml].mean(0), axis=1) < 0.06]; mb_xs = len(cross_intersections(hp, T, mp, mt, near))
    lidlip = np.flatnonzero((mk['bucket']['upper_eyelid_r'] | mk['bucket']['lower_eyelid_r'] | mk['bucket']['upper_eyelid_l'] | mk['bucket']['lower_eyelid_l'] | mk['bucket']['upper_lip'] | mk['bucket']['lower_lip'] | mk['bucket']['mouth_corner_l'] | mk['bucket']['mouth_corner_r'])[T].any(1) & mk['outer_faces'])
    self_xs = len(FQ.self_intersections(hp, T, candidates=lidlip, merge_groups=dup)) if lid_faces_self else None
    gates = {'attachment_exact': att == 0.0, 'seams_coincident': seam == 0.0, 'no_new_collapsed_triangles': q['new_collapsed_triangles'] == 0, 'inverted_not_worse_than_donor_plus_10_excluding_lash': inv_fit <= inv_donor + 10,
             'no_lid_record_inside_placed_cap_within_extent': all(e['inside_actual_cap_surface'] == 0 for e in eyes.values()), 'mouth_loop_not_collapsed': h_fit > 0.5 * h_don}
    return {'gates': gates, 'all_gates_pass': all(gates.values()), 'attachment_max_error_mm': att * 1000, 'seam_max_gap_mm': seam * 1000, 'triangle_quality_vs_donor': q, 'inverted_excluding_lash': {'fit': inv_fit, 'donor': inv_donor},
            'eyes': {k: {kk: v[kk] for kk in ('inside_sphere', 'inside_sphere_within_cap_extent', 'inside_actual_cap_surface', 'min_signed_to_sphere_mm', 'min_signed_to_cap_surface_within_extent_mm', 'penetrating_depth_mm', 'head_cap_triangle_intersections', 'cap_records_outside_skin', 'cap_apex_outside_skin_mm', 'cap_rim_depth_under_skin_mm')} for k, v in eyes.items()},
            'lips': {'slit_px_front_zoom': slit, 'mouth_loop_height_mm': {'donor': h_don * 1000, 'fit': h_fit * 1000}, 'mouthbag_head_intersections': mb_xs}, 'haircap': {k: hair[k] for k in ('covered', 'poking', 'open', 'poking_above_0p5mm', 'poking_above_2mm', 'under_cap_below_minus_5mm', 'head_cap_triangle_intersections', 'under_cap_signed_clearance_mm')},
            'lid_lip_self_intersections_outer_faces': self_xs, 'socket': socket_contact(ex_donor, ex_cand, mk)['summary']}
