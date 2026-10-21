"""General neck/jaw transition with destination boundary constraints (port of experiments/claude/neck v3, plus the
phase-2 repairs: seam-consistent ear/oral sets, confident-target outlier demotion, fold-aware relaxation).

Inputs are in the Blender frame (see package docstring). `fit` is an already-corresponded head (e.g. the registered
UV fit) whose face above `fit_top` is trusted; below it this module rebuilds the neck from the source surface.

Phase-2 changes (all default ON; pass seam_consistent_sets=False, outlier_mm=None, fold_relax_iters=0 for v3):
  * ear_ids / oral_ids are expanded across seam-duplicate groups. In v3 a UV-defined ear vertex whose seam twin was
    not in the set received two different displacements and the final seam averaging deformed the ear boundary.
  * A confident spatial target whose signed travel along its normal differs from the median travel of its confident
    neighbours by more than `outlier_mm` is demoted to free (harmonic). Normal-direction rays at the chin tip hit either the
    chin front or the underside, giving neighbours 10-20 mm apart targets and a folded skin.
  * After solving, faces that are locally inverted / folded (>`relax_dihedral_deg`) or that intersect other skin
    faces and were not so in `fit` mark their confident vertices (+1 ring) free; the field is re-solved. Repeated up
    to `fold_relax_iters` times or until no introduced defect remains. Frozen, fit, oral and ear vertices are never
    demoted, so attachment, face, oral and ear geometry are unchanged by the relaxation.
  * Mouth-interior vertices that lie on no outer-shell face (donor occlusion test) but are not in the Studio's oral id
    list follow the fit exactly (`interior_follows_fit`). In v3 they were treated as skin and their normal rays pulled
    them onto the CC5 skin surface, producing skin/interior intersections at the chin.
  * The ear is no longer split by fit_top: all ear vertices keep the fit plus the rigid-follow translation, which is
    now measured on the neck-band side of the ear ring only and blended 1 -> 0 across the ear towards the trusted-fit
    boundary (`ear_blend`), so neither ear boundary has a step.
  * Skin within `lip_rings` graph rings of the mouth interior keeps the fit. The lower lip of these heads dips below
    band_top, and in v3 its margin received skin ray targets (slivers at the mouth corners).
  * Rays are cast from the fit position (`ray_from='fit'`), not the donor position, so the tangential slide the
    registered fit applies inside the Studio blend band is inherited and only the normal error is corrected. With
    donor-origin rays Demitra's lower lip (fit slid 7-13 mm along the surface) met a band with no slide: folds.
  * Phase-3 (lip-margin transition): a band vertex whose *fit* position already lies within `lip_pin_mm` of the source
    surface and within `lip_pin_rings` graph rings of the mouth interior is pinned to the fit (mode 3). The registered
    fit owns the lips and their surroundings wherever it already agrees with the source; only off-surface skin is
    corrected. In v4 such vertices were confident targets with ~0 travel that the fold relaxation later freed, and the
    harmonic fill then dragged them 0.7-2.6 mm off the surface (five folded/sliver/intersecting faces under Turgeon's
    right lower lip). Default `lip_pin_rings=None` pins anywhere in the band (measured best: 0/0/1 introduced skin
    faces on Turgeon/Fedorov/Demitra vs 0/1/1 with 4 rings and 5/1/0 for v4); the pinned set only contains vertices
    whose correction would have been < lip_pin_mm anyway, so jaw distances are unchanged (medians equal to 0.03 mm).
  * Phase-3 (`relax_keep_best`): the fold relaxation keeps the solution with the fewest introduced skin defects
    across its iterations instead of the last one (on Turgeon iteration 2 raised the count from 1 to 12).
  * Surface snap (`snap_mm`, default OFF): optional post-projection of band vertices onto the nearest CC5 point.
    Measured on the three fixtures it increased skin/skin overlaps under the lower lip (8-12 new pairs) and is kept
    only as an experiment switch.
"""
import numpy as np
from . import geometry as G
from . import face_quality as FQ


def _expand_groups(mask, dup):
    mask = mask.copy()
    for ids in dup:
        if mask[ids].any(): mask[ids] = True
    return mask


def neck_transition(orig, fit, tris, src, stris, *, oral_ids=(), ear_ids=(), keep_rings=2, band_top=1.775,
                    fit_top=1.79, max_delta=0.03, shape_rings=10, cc5_bottom_margin=0.015, smooth_passes=4,
                    seam_consistent_sets=True, outlier_mm=8.0, fold_relax_iters=3, relax_dihedral_deg=150.0,
                    relax_rings=2, interior_follows_fit=True, ear_blend=True, lip_rings=2, snap_mm=None, ray_from='fit',
                    lip_pin_mm=0.5, lip_pin_rings=None, relax_keep_best=True, interior_mode='fit', lining_blend_rings=3):
    orig = np.asarray(orig, float); fit = np.asarray(fit, float); tris = np.asarray(tris); n = len(orig)
    dup = G.seam_groups(orig); adj = G.adjacency(n, tris, dup); nb = [np.array(sorted(adj[i])) for i in range(n)]
    frozen = G.grow(G.bottom_loop(orig, tris), adj, keep_rings)
    is_frozen = np.zeros(n, bool); is_frozen[frozen] = True
    is_oral = np.zeros(n, bool); is_oral[list(oral_ids)] = True
    is_ear = np.zeros(n, bool); is_ear[list(ear_ids)] = True
    if seam_consistent_sets: is_oral = _expand_groups(is_oral, dup); is_ear = _expand_groups(is_ear, dup)
    skin_faces = FQ.outer_shell_faces(orig, tris) if (fold_relax_iters or interior_follows_fit) else None
    if interior_follows_fit:
        on_skin = np.zeros(n, bool); on_skin[np.unique(tris[skin_faces])] = True
        interior = ~on_skin & ~is_frozen & ~is_ear
        if seam_consistent_sets: interior = _expand_groups(interior, dup)
        if interior_mode in ('harmonic', 'rigid_blend'):
            # phase 4 (mouth lining): the occluded lining that is NOT in the oral-id list stays FREE and is solved
            # harmonically between the oral islands (Studio rigid mouth delta) and the lip margin (fit). With
            # interior_mode='fit' it copies the registered UV fit, which on Turgeon collapsed the inner-upper-lip
            # lining onto the teeth (two patches of lining through the upper teeth, user-reported in game).
            lining = interior & ~is_oral   # occluded lining only; the oral-id islands keep the Studio rigid delta
        is_oral |= interior
    soup = G.TriangleSoup(src, stris); zmin = np.asarray(src)[:, 2].min()
    vn = G.vertex_normals(orig, tris, dup)
    D = np.zeros((n, 3)); known = is_frozen.copy(); mode = np.zeros(n, int); mode[is_frozen] = 1
    above = (orig[:, 2] >= fit_top) & ~is_frozen
    if ear_blend: above &= ~is_ear
    D[above] = fit[above] - orig[above]; known[above] = True; mode[above] = 3
    D[is_oral] = fit[is_oral] - orig[is_oral]; known[is_oral] = True; mode[is_oral] = 4
    lining_n = 0
    if interior_follows_fit and interior_mode == 'harmonic':
        known[lining] = False; mode[lining] = 0; D[lining] = 0; lining_n = int(lining.sum())
    elif interior_follows_fit and interior_mode == 'rigid_blend':
        # lining follows the oral islands' rigid delta (donor lining/teeth arrangement) and blends into the lip
        # margin (fit) over `lining_blend_rings` graph rings from the wet line, so the mouth stays continuous
        oral_orig = np.zeros(n, bool); oral_orig[list(oral_ids)] = True
        md = (fit[oral_orig] - orig[oral_orig]).mean(0)
        wet = [int(i) for i in np.flatnonzero(lining) if any((not lining[j]) and (not is_oral[j]) for j in adj[i])]
        rd = G.ring_distance(wet, adj, n, lining_blend_rings); w = np.clip(np.where(np.isfinite(rd), rd, lining_blend_rings) / lining_blend_rings, 0, 1); w = w * w * (3 - 2 * w)
        L = np.flatnonzero(lining); D[L] = (1 - w[L])[:, None] * (fit[L] - orig[L]) + w[L][:, None] * md; known[L] = True; mode[L] = 4; lining_n = int(len(L))
    # lip margin: skin within `lip_rings` of the mouth interior keeps the fit (the registered UV fit owns the lips;
    # a height band alone catches the lower lip on these heads)
    lip = np.zeros(n, bool)
    if lip_rings and is_oral.any():
        lip[G.grow(np.flatnonzero(is_oral), adj, lip_rings)] = True; lip &= ~known & ~is_ear
        if interior_follows_fit and interior_mode in ('harmonic', 'rigid_blend'): lip &= ~lining
        D[lip] = fit[lip] - orig[lip]; known[lip] = True; mode[lip] = 3
    exclude_lining = lining if (interior_follows_fit and interior_mode in ('harmonic', 'rigid_blend')) else np.zeros(n, dtype=bool)
    band = np.flatnonzero((orig[:, 2] < band_top) & ~known & ~is_ear & ~exclude_lining); tdist = np.full(n, np.nan)
    for i in band:
        # ray origin: the fit position (default) so the registration's tangential slide inside the Studio blend band
        # is inherited and only the normal error is corrected; 'donor' reproduces v3 (no tangential component,
        # which left a 7-13 mm tangential step under Demitra's lower lip)
        p = fit[i] if ray_from == 'fit' else orig[i]; d = vn[i]; best = None
        for sgn in (1.0, -1.0):
            t = soup.ray_hit(p, d * sgn)
            if t is None: continue
            hp = p + d * sgn * t
            if hp[2] < zmin + cc5_bottom_margin: continue
            if best is None or t < best[0]: best = (t, hp)
        if best is None: continue
        delta = best[1] - p; tdist[i] = np.linalg.norm(delta)
        if tdist[i] > max_delta: continue
        D[i] = best[1] - orig[i]; known[i] = True; mode[i] = 2
    # --- lip-margin pinning: confident targets that already sit on the source surface near the mouth keep the fit
    pinned = np.zeros(n, bool)
    if lip_pin_mm is not None and is_oral.any():
        near = np.ones(n, bool) if lip_pin_rings is None else np.isfinite(G.ring_distance(np.flatnonzero(is_oral), adj, n, lip_pin_rings))
        # nearest-surface distance of the FIT position (not the ray travel: a fit point exactly on the surface gets no
        # valid ray hit and would otherwise be 'no hit' -> free)
        for i in band[near[band]]:
            if mode[i] in (0, 2) and soup.nearest(fit[i])[1] <= lip_pin_mm / 1000: pinned[i] = True
        for i in np.flatnonzero(pinned): D[i] = fit[i] - orig[i]; mode[i] = 3; known[i] = True
    # --- outlier demotion among confident targets
    demoted_outliers = []
    if outlier_mm is not None:
        conf = mode == 2; s = ((D - (fit - orig) * (ray_from == 'fit')) * vn).sum(1)   # signed ray travel along the normal
        for i in np.flatnonzero(conf):
            nn = nb[i][conf[nb[i]]]
            if len(nn) < 2: continue
            med = np.median(s[nn])
            # one-sided: demote the vertex that travels farther than its neighbours (a wrong ray hit), never one that
            # is already closer to the surface than they are (its small target is the trustworthy one)
            if abs(s[i] - med) > outlier_mm / 1000 and abs(s[i]) > abs(med): demoted_outliers.append(int(i))
        for i in demoted_outliers: mode[i] = 0; known[i] = False; D[i] = 0
    ear_all = np.flatnonzero(is_ear); Ts = {}; ear_w = np.ones(n)
    if ear_blend and len(ear_all):
        # scalar blend over the ear graph: 1 next to the neck-band boundary, 0 next to the trusted-fit boundary
        up = (orig[:, 2] >= fit_top) & ~is_ear
        wk = np.zeros(n, bool); ear_w[:] = 0.0
        for i in ear_all:
            outside = [j for j in adj[i] if not is_ear[j]]
            if not outside: continue
            if any(not up[j] for j in outside): ear_w[i] = 1.0; wk[i] = True
            elif any(up[j] for j in outside): ear_w[i] = 0.0; wk[i] = True
        nb_ear = [np.array([j for j in nb[i] if is_ear[j]], int) for i in range(n)]
        free_e = [int(i) for i in ear_all if not wk[i] and len(nb_ear[i])]
        W = ear_w[:, None].copy(); G.laplace_fill(W, free_e, nb_ear); ear_w = W[:, 0]
        ear_w = np.clip(ear_w, 0, 1); ear_w = ear_w * ear_w * (3 - 2 * ear_w)

    def solve(D, known, mode):
        D = D.copy(); known = known.copy(); mode = mode.copy()
        it1 = G.laplace_fill(D, np.flatnonzero(~known), nb)
        for side in (1, -1):
            ids = ear_all[orig[ear_all, 0] * side > 0]
            ring = np.array(sorted({j for i in ids for j in adj[i] if not is_ear[j]}), int)
            if ear_blend and len(ring) and (orig[ring, 2] < fit_top).any(): ring = ring[orig[ring, 2] < fit_top]
            T = (D[ring] - (fit[ring] - orig[ring])).mean(0) if len(ring) else np.zeros(3); Ts[side] = T
            D[ids] = (fit[ids] - orig[ids]) + T * ear_w[ids, None]; known[ids] = True; mode[ids] = 5
        it2 = G.laplace_fill(D, np.flatnonzero(~known), nb)
        smooth_ids = np.flatnonzero((mode == 2) | (mode == 0))
        for _ in range(smooth_passes):
            Dn = D.copy()
            for i in smooth_ids: Dn[i] = 0.5 * D[i] + 0.5 * D[nb[i]].mean(0)
            D = Dn
        w = np.clip(G.ring_distance(frozen, adj, n, shape_rings) / shape_rings, 0, 1); ws = w * w * (3 - 2 * w)
        result = orig + D * ws[:, None]
        result[above] = fit[above]
        if snap_mm:
            # surface snap: the harmonic fill and the smoothing passes move band vertices off the CC5 surface; where
            # the result is within snap_mm of the surface (and above the CC5 cut, and outside the collar shaping
            # zone) put it back on the nearest surface point. Tangential distribution stays the smooth one.
            for i in np.flatnonzero(((mode == 2) | (mode == 0)) & (ws > 0.999)):
                q, d = soup.nearest(result[i])
                if d < snap_mm / 1000 and q[2] >= zmin + cc5_bottom_margin: result[i] = q
        for ids in dup: result[ids] = result[ids].mean(0)
        result[frozen] = orig[frozen]
        return result, mode, [it1, it2]

    result, mode_out, its = solve(D, known, mode)
    # --- fold-aware relaxation
    fit_m = FQ.face_metrics(fit, tris, dup, dihedral_deg=relax_dihedral_deg) if fold_relax_iters else None
    skin = (skin_faces & ~is_oral[tris].any(1)) if fold_relax_iters else None
    relax_log = []; demoted_relax = set(); best = None; chosen = None
    for k in range(fold_relax_iters + 1 if fold_relax_iters else 0):
        m = FQ.face_metrics(result, tris, dup, dihedral_deg=relax_dihedral_deg)
        bad = ((m['inverted_local'] & ~fit_m['inverted_local']) | (m['folded_edge'] & ~fit_m['folded_edge'])) & skin
        movable = ((mode_out[tris] == 2) | (mode_out[tris] == 0)).any(1); bad &= movable
        cand = np.flatnonzero(movable & skin)
        inter = FQ.self_intersections(result, tris, candidates=cand, merge_groups=dup)
        inter_fit = set(FQ.self_intersections(fit, tris, candidates=cand, merge_groups=dup))
        for i, j in inter:
            if (i, j) not in inter_fit and skin[i] and skin[j]: bad[i] = True; bad[j] = True
        bad_faces = np.flatnonzero(bad)
        if best is None or len(bad_faces) < best[0]: best = (len(bad_faces), result.copy(), mode_out.copy(), list(its), k)
        if k == fold_relax_iters: relax_log.append({'iteration': k + 1, 'introduced_defect_faces': int(len(bad_faces)), 'faces': bad_faces.tolist(), 'demoted_confident_vertices': 0, 'final_check': True}); break
        verts = set(int(v) for v in tris[bad_faces].ravel())
        verts = set(G.grow(verts, adj, relax_rings).tolist()) if verts else set()
        targets = [v for v in verts if mode[v] == 2]
        relax_log.append({'iteration': k + 1, 'introduced_defect_faces': int(len(bad_faces)), 'faces': bad_faces.tolist(), 'demoted_confident_vertices': len(targets)})
        if not targets: break
        for v in targets: mode[v] = 0; known[v] = False; D[v] = 0; demoted_relax.add(v)
        result, mode_out, its = solve(D, known, mode)
    if relax_keep_best and best is not None and best[4] != len(relax_log) - 1 and best[0] < relax_log[-1]['introduced_defect_faces']:
        _, result, mode_out, its, chosen = best
    conf = np.flatnonzero(mode == 2)
    report = {'frozen_vertices': int(len(frozen)), 'band_candidates': int(len(band)), 'confident_spatial': int(len(conf)),
              'rejected_no_hit': int(np.isnan(tdist[band]).sum()), 'rejected_over_max_delta': int((~np.isnan(tdist[band]) & (tdist[band] > max_delta)).sum()),
              'demoted_outliers': len(demoted_outliers), 'demoted_by_fold_relaxation': len(demoted_relax), 'relaxation': relax_log, 'relaxation_kept_iteration': (chosen + 1) if chosen is not None else None, 'lip_pinned_vertices': int(pinned.sum()),
              'interior_vertices_following_fit': int(is_oral.sum() - len(set(oral_ids))) if interior_follows_fit else 0, 'interior_mode': interior_mode, 'lining_vertices_harmonic': lining_n, 'ear_vertices': int(len(ear_all)), 'lip_margin_vertices_kept_fit': int(lip.sum()),
              'laplace_iterations': its, 'ear_translation_m': {int(k): v.tolist() for k, v in Ts.items()},
              'confident_delta_m': {'median': float(np.median(tdist[conf])), 'max': float(tdist[conf].max())} if len(conf) else None,
              'crown': float(result[:, 2].max()), 'donor_crown': float(orig[:, 2].max()),
              'frozen_max_error': float(np.abs(result[frozen] - orig[frozen]).max()),
              'seam_max_gap': max((float(np.linalg.norm(result[ids] - result[ids[0]], axis=1).max()) for ids in dup), default=0.0),
              **G.triangle_quality(fit, result, tris)}
    return result, mode_out, report
