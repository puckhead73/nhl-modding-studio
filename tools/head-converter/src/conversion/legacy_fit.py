"""Bounded CC5 -> Legacy donor-fit experiment core (numpy only). Research code with receipts, not the accepted NHL21
recipe: the Legacy head is a front shell open at the back under the native haircap, its mouth is a hole closed by a
separate mouthbag, and its eyes are separate spherical caps, so the NHL21 stage set does not apply verbatim.

Three correspondence hypotheses are kept separately (all evaluated by `legacy_fit_experiment.py`):
  atlas_a         the Studio 45-pair TPS (NHL atlas-A -> CC5 atlas) applied to the Legacy UVs. Justified only by the
                  measured template identity (every player_67 UV within 0.4 px @1024 of an atlas-A twin); a
                  hypothesis to test, never assumed.
  legacy_surface  a Legacy-specific TPS solved from the donor's own records: after eye-centre alignment each record's
                  nearest CC5 surface point gives (Legacy UV -> CC5 UV) pairs; anchors are chosen by farthest-point
                  sampling on the face window; residuals are reported on the held-out records.
  surface         no UV registration at all: nearest-surface projection after alignment (bounded, no smoothing beyond
                  the seam/frozen rules), the reference the two registrations are compared against.
Protected: attachment loop (+ rings) exact; seam groups coincident; companions (eyes, mouthbag, haircap) move only as
rigid translations declared in the report. Frame: Blender (Z up) inside; native in/out through legacy_exchange.
"""
import time
import numpy as np
from . import geometry as G, face_quality as FQ, uv_fit as F, masks as M, legacy_exchange as LX, cc5_source as C
from .eye_fit import sphere_fit
from . import crease as CR

DEFAULTS = dict(band_height=0.05, rings=2, miss_px=1.5, raster=2048, rear_y=0.005, rear_blend_rings=4, anchors=60, anchor_max_mm=2.0, smooth_passes=3, mouth_gap_policy='follow_source', mouth_gap_rings=2, lip_relax_rings=0, lip_relax_passes=0, lip_relax_alpha=0.5,
                lid_clearance_mm=None, lid_clearance_placement='cc5_centre', scalp_policy='fit', scalp_blend_rings=4, scalp_max_gap_mm=30.0, scalp_rim_tol_mm=3.0, scalp_rim_below_eyes_mm=None, lip_block_rings=3, lip_block_blend_rings=3, lip_block_blend_passes=4,
                lip_block_grid='none', lip_blend_separation='none', lip_blend_separation_passes=3, lip_side_radius_m=0.004, lip_crossing_max_iters=8, lip_block_blend_mode='laplace',
                lid_clearance_scope='buckets', socket_rings=2, socket_rim_policy='none', socket_rim_tol_mm=1.0, socket_radius_rings=0, socket_radius_graded=True, socket_joint_rings=3, socket_joint_iters=80, socket_joint_floor_mm=1.0, socket_block_rings=2, socket_block_blend_rings=3,
                cap_centre_override=None, socket_con_rings=0, socket_con_support_clear_mm=2.0, socket_con_floor_mm=1.0, socket_con_orient_min_dot=0.5, socket_con_area_ratio=0.5, socket_con_w_smooth=1.0, socket_con_w_rim=10.0, socket_con_w_floor=10.0, socket_con_w_orient=10.0, socket_con_w_area=1.0, socket_con_w_edge=1.0, socket_con_iters=400, socket_con_tol=1e-7, socket_con_w_crease=0.0, socket_con_crease_margin_deg=10.0, socket_con_init=None, socket_con_lattice=False, socket_con_lattice_passes=6, socket_con_crease_reference='donor_fit_input', socket_con_crease_sign_deg=15.0, socket_con_crease_free_deg=None, cap_radius_scale=None)
# cap_radius_scale: None = the native caps keep their radius (every recipe before 2026-09-16); else [s_r, s_l] = DECLARED isotropic scale of each eye cap about its own fitted sphere centre (assemble applies it after the translation; fit reads the scaled radius r0*s wherever a cap radius enters: lid clearance, preserve_donor_radius, constrained rim target/floor). Rim targets keep the DONOR SIGNED DISTANCE to the cap surface (donor_r + r0*(s-1)), so a lid margin resting on the donor cap rests on the scaled cap. Refused (ValueError) with joint_smooth / donor_block, which were never generalised to a scaled cap.
# socket_con_crease_reference: 'donor_fit_input' = per-edge [min, max](donor, fit input) +- margin (crease-01..03 behaviour); 'donor_signed' = crease.envelope_donor_signed: donor +- margin, sign kept on edges with |donor| >= socket_con_crease_sign_deg (the single anatomical reference policy of the cap-radius study; the judge uses the same envelope)
# socket_con_lattice: after the continuous solve, a storage-lattice stage (lattice_stage): the whole head is snapped to native half-float storage (the eye region's vertical step is ~0.98 mm at y_up ~1.83 m) and every region record's stored value is chosen per axis among its floor / ceil / +-1 step by coordinate descent on the SAME energy evaluated on stored coordinates - the free variables become the stored values, so what is judged (predicted native) is what was optimised
# socket_con_w_crease: weight of the coupled signed-dihedral term (crease.py) over the support + surround edges, envelope per edge = [min, max] of the DONOR and FIT-INPUT dihedrals +- margin; 0 = term off (the 2026-09-15 eyealign-01 behaviour). socket_con_init: full head positions (Blender m) to warm-start the region from (None = the fit input)
# cap_centre_override: None = the placed caps sit on the (shifted) CC5 eye centres (frozen recipe); else [[x,y,z],[x,y,z]] Blender metres for eye_r/eye_l - a DECLARED cap placement (alignment experiment A2: level cap heights without rotating the source)
# socket_con_*: the 'constrained' socket policy (constrained_socket_refit): rings 0 = support chosen from geometry (grow rows until every row record is >= socket_con_support_clear_mm outside the placed cap, +1 boundary row), floor = row clearance outside the cap, orient_min_dot = minimum normal.reference dot per face, area_ratio = minimum face area / reference area, w_* = energy weights (mm units), iters / tol = L-BFGS budget and stopping rule
LIP_BLOCK_BLEND_MODES = ('laplace', 'graded')             # laplace: run-17's 0.5-weight neighbour averaging of the blend rings (fitted skin pulled toward the block: the lower-lip skin ends 0.2 mm from the upper-lip block edge); graded: ring k of R keeps the rigid donor translate with weight (R-k+1)/(R+1) and the fitted position otherwise, no smoothing (the donor's lip separation decays over the rings instead of collapsing at ring 1)
LIP_BLOCK_GRIDS = ('none', 'native_half')                       # native_half: the rigid lip-block translation is snapped to the coarsest half-float ulp among the block records per axis so every translated donor record (and the mouthbag, which follows the same translation) is exactly representable in native storage
LIP_BLEND_SEPARATIONS = ('none', 'donor', 'storage', 'donor_side', 'crossing_driven')  # blend-ring records vs the rigid block after the blend: 'donor'/'storage' push away from the nearest block point to the donor's own separation / 2x the half-float storage bound (both measured: they create new fold crossings); 'donor_side' restores the donor's side of every local block-face plane by min(2x storage bound, donor distance) along that plane normal. All re-checked on predicted native coordinates
SCALP_POLICIES = ('fit', 'preserve_under_haircap', 'preserve_haircap_rim')
# player_67's haircap is a hair volume 0-8 mm above the covered scalp whose open rim rests on the skin (nape, sideburns,
# hairline). 'preserve_under_haircap' freezes the whole covered scalp (460 records) and its blend band inverted 40 faces
# on Turgeon (runs 10/11: evidence-only); 'preserve_haircap_rim' freezes only the rim contact skin (+1 ring) and blends.
MOUTH_GAP_POLICIES = ('follow_source', 'preserve_donor_gap', 'preserve_donor_lips')
# route-specific, declared per job. player_67's mouth is CLOSED in projection: the 32-record mouth loop is the inner lip
# contour 16 mm deep inside the mouth (13.7 mm tall), not a visible gap. follow_source maps that contour onto the CC5
# closed lip line (collapses it, inverts lip faces); preserve_donor_gap keeps the loop rigid at the fitted centroid
# (opens a visible slit between the CC5 lips); preserve_donor_lips keeps the donor's whole lip block (loop + N rings)
# rigid, translated by its mean fitted displacement, and blends the rings outside it: donor lips, Turgeon face.
EYE_PLACEMENTS = ('translate_to_cc5_centre', 'untouched')      # caps are head-rigid on player_67 (palette 8 = Head, 100 %): a translation is a shape offset, not a pivot change
LID_CLEARANCE_PLACEMENTS = ('cc5_centre', 'untouched')         # which placed cap the lid-clearance rule reads: the cap translated to the CC5 centre, or the donor cap left where it is
LID_CLEARANCE_SCOPES = ('buckets', 'socket_rings')             # buckets: run-17's eyelid palette buckets only (the upper-margin rows of player_67 are weighted on the eye_control palettes and were never cleared); socket_rings: buckets + every outer-shell record within `socket_rings` rings of the socket rim (topology, any palette), lash band still excluded
SOCKET_RIM_POLICIES = ('none', 'preserve_donor_radius', 'joint_smooth', 'donor_block', 'constrained')
CREASE_REFERENCES = ('donor_fit_input', 'donor_signed')


def cap_radius_scales(p):
    """Declared per-side cap scales [s_r, s_l] from the params (None -> [1, 1]); validated: two finite positive numbers."""
    sc = p.get('cap_radius_scale')
    if sc is None: return np.ones(2)
    sc = np.asarray(sc, float)
    if sc.shape != (2,) or not np.isfinite(sc).all() or (sc <= 0).any(): raise ValueError(f'cap_radius_scale must be [s_r, s_l] > 0, got {p.get("cap_radius_scale")!r}')
    return sc   # constrained: bounded wider eye-region refit = one energy over rim + K rows with explicit rim-radius / clearance-floor / face-orientation / face-area / edge-length terms (constrained_socket_refit); no uniform radial push, no unconstrained Laplacian sweep   # donor_block: rim + socket_block_rings rows keep the DONOR shape, translated rigidly by the cap's own translation (exact donor rim/cap relation), graded blend over socket_block_blend_rings into the fitted face (eye-shape cost: the donor's eye opening, not the source's)        # preserve_donor_radius: every socket-rim (atlas lash-band) record keeps its DONOR distance from the cap centre, measured against the PLACED cap (direction from the fitted position, radius from the donor): the lid margin that rests on the donor eyeball rests on the placed eyeball; the socket-fan apex keeps its donor depth


# ---- alignment ---------------------------------------------------------------------------------------------
def eye_centres_cc5(src):
    E = np.array(C.mesh(src, C.EYE)['positions']); mats = C.eye_material_ids(src)
    cR, rR, _ = sphere_fit(E[mats['Std_Eye_R']]); cL, rL, _ = sphere_fit(E[mats['Std_Eye_L']])
    return np.array([cR, cL]), float((rR + rL) / 2)


def eye_centres_legacy(ex):
    out = []; rad = []
    for role in ('eye_r', 'eye_l'):
        c, r, _ = sphere_fit(G.native_to_blender(LX.submesh(ex, role)['positions'])); out.append(c); rad.append(r)
    return np.array(out), float(np.mean(rad))


def align_shift(source_eyes, donor_eyes):
    """Translation applied to the CC5 source so its eye-centre midpoint lands on the donor's (all three axes; the
    Legacy head has no crown to align on)."""
    return np.asarray(donor_eyes).mean(0) - np.asarray(source_eyes).mean(0)


def level_rotation(source_eyes, donor_eyes):
    """Minimal rigid rotation taking the source eye line (R->L) onto the donor eye line direction. Returns (R 3x3,
    angle_deg). Declared alignment option: the CC5 scan's eye line may be rolled relative to the donor's level caps;
    a rotation about the eye midpoint removes that without any per-eye translation."""
    a = np.asarray(source_eyes[1], float) - np.asarray(source_eyes[0], float); b = np.asarray(donor_eyes[1], float) - np.asarray(donor_eyes[0], float)
    a /= np.linalg.norm(a); b /= np.linalg.norm(b); v = np.cross(a, b); s = np.linalg.norm(v); c = float(a @ b)
    if s < 1e-12: return np.eye(3), 0.0
    K = np.array([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]]); R = np.eye(3) + K + K @ K * ((1 - c) / s ** 2)
    return R, float(np.degrees(np.arctan2(s, c)))


def rotate_source(src, R, pivot):
    """Copy of a CC5 source geometry with every mesh rotated rigidly by R about `pivot` (Blender frame)."""
    R = np.asarray(R, float); pivot = np.asarray(pivot, float); out = []
    for m in src:
        mm = dict(m); mm['positions'] = ((np.asarray(m['positions'], float) - pivot) @ R.T + pivot).tolist(); out.append(mm)
    return out


def scale_eye_regions(src, centres, k, r0_m=0.017, R_m=0.025):
    """Copy of a CC5 source geometry with every mesh scaled RADIALLY about each eye centre by k inside r0, fading
    (smoothstep) to 1 at R: an anatomy normalisation that carries the lid surface with the eyeball (scaling the
    eyeball mesh alone would not move the skin). Declared experiment (alignment A3); the likeness cost = the
    displacement of the source surface, reported by the caller. Returns (source copy, report)."""
    centres = np.asarray(centres, float); out = []; moved = []
    for m in src:
        X = np.asarray(m['positions'], float).copy(); X0 = X.copy()
        for c in centres:
            v = X0 - c; d = np.linalg.norm(v, axis=1); w = np.clip((d - r0_m) / max(R_m - r0_m, 1e-9), 0, 1); w = 1 - (w * w * (3 - 2 * w)); sc = 1 + (k - 1) * w; sel = w > 0
            X[sel] = c + (X[sel] - c) * sc[sel, None]          # weights from the ORIGINAL distances; vertices beyond R are left bit-identical
        mm = dict(m); mm['positions'] = X.tolist(); out.append(mm); moved.append(np.linalg.norm(X - X0, axis=1))
    mv = np.concatenate(moved) if moved else np.zeros(0)
    return out, {'k': float(k), 'r0_mm': r0_m * 1000, 'R_mm': R_m * 1000, 'vertices_moved': int((mv > 1e-9).sum()), 'max_move_mm': float(mv.max() * 1000) if len(mv) else 0.0, 'note': 'radial scale about each CC5 eye centre; lid surface carried; source original untouched (copy)'}


def placed_cap_centres(SE, shift, override):
    """Blender-frame centres the eye caps are translated to: the shifted CC5 centres, or a declared override."""
    return np.asarray(override, float) if override is not None else np.asarray(SE, float) + np.asarray(shift, float)


# ---- correspondences ---------------------------------------------------------------------------------------
def _bary(q, a, b, c):
    v0 = b - a; v1 = c - a; v2 = q - a; d00 = v0 @ v0; d01 = v0 @ v1; d11 = v1 @ v1; d20 = v2 @ v0; d21 = v2 @ v1; den = d00 * d11 - d01 * d01 + 1e-30
    bv = (d11 * d20 - d01 * d21) / den; bw = (d00 * d21 - d01 * d20) / den; return np.array([1 - bv - bw, bv, bw])


def surface_correspondence(P, S, ST, SU_td):
    """Nearest CC5 surface point per record: returns (points, distances, faces, cc5 uv (top-down))."""
    soup = G.TriangleSoup(S, ST); n = len(P); Q = np.zeros_like(P); D = np.zeros(n); K = np.zeros(n, int); UV = np.zeros((n, 2))
    for i in range(n):
        q, d, k = soup.nearest_face(P[i]); Q[i] = q; D[i] = d; K[i] = k; b = np.clip(_bary(q, *S[ST[k]]), 0, 1); b /= b.sum(); UV[i] = b @ SU_td[ST[k]]
    return Q, D, K, UV


def tps_from_pairs(src_uv, dst_uv, lam=1e-6):
    """Solve TPS coefficients mapping src_uv -> dst_uv (both in [0,1]); layout compatible with uv_fit.tps_warp."""
    a = np.asarray(src_uv, float); t = np.asarray(dst_uv, float); n = len(a)
    r2 = ((a[:, None, :] - a[None, :, :]) ** 2).sum(2); K = r2 * np.log(np.maximum(r2, 1e-20)); Pm = np.c_[np.ones(n), a]
    A = np.block([[K + np.eye(n) * lam, Pm], [Pm.T, np.zeros((3, 3))]]); return np.linalg.solve(A, np.vstack([t, np.zeros((3, 2))]))


def farthest_point_subset(X, k, seed=0):
    X = np.asarray(X, float); n = len(X)
    if n <= k: return np.arange(n)
    rng = np.random.default_rng(seed); chosen = [int(rng.integers(n))]; d = np.linalg.norm(X - X[chosen[0]], axis=1)
    for _ in range(k - 1):
        j = int(np.argmax(d)); chosen.append(j); d = np.minimum(d, np.linalg.norm(X - X[j], axis=1))
    return np.array(chosen)


def registration_from_surface(U, cc5_uv, dist, eligible, n_anchors=60, max_dist=2e-3, seed=0):
    """Legacy-specific TPS from surface correspondences: anchors = farthest-point subset of eligible records whose
    nearest-surface distance is below max_dist. Returns (anchors uv, coefficients, report)."""
    ok = eligible & (dist <= max_dist); ids = np.flatnonzero(ok)
    if len(ids) < 8: raise ValueError(f'only {len(ids)} eligible records for a Legacy registration')
    sel = ids[farthest_point_subset(U[ids], n_anchors, seed)]; coef = tps_from_pairs(U[sel], cc5_uv[sel])
    held = np.setdiff1d(ids, sel); pred = F.tps_warp(U[sel], coef, U[held]); res = np.linalg.norm(pred - cc5_uv[held], axis=1) * 4096
    return U[sel], coef, {'anchors': int(len(sel)), 'eligible_records': int(len(ids)), 'max_dist_mm': max_dist * 1000, 'held_out_residual_px_at_4096': {'median': float(np.median(res)), 'p95': float(np.percentile(res, 95)), 'max': float(res.max())}, 'anchor_ids': sel.tolist()}


def sample_by_uv(mapped, S, ST, SU_td, raster, miss_px):
    """Barycentric sample of the CC5 surface at mapped (top-down) UVs; misses snapped to the nearest chart triangle."""
    R = raster; tri_id = F.uv_raster(SU_td, ST, R); n = len(mapped); mapped = mapped.copy()
    qi = np.clip((mapped * R).astype(int), 0, R - 1); hit = tri_id[qi[:, 1], qi[:, 0]]; uv_dist = np.zeros(n)
    soup_uv = G.TriangleSoup(np.c_[SU_td, np.zeros(len(SU_td))], ST)
    for i in np.flatnonzero(hit < 0):
        q, d, k = soup_uv.nearest_face(np.array([mapped[i, 0], mapped[i, 1], 0.0])); hit[i] = k; uv_dist[i] = d; mapped[i] = q[:2]
    B = np.zeros((n, 3))
    for i in range(n):
        a, b, c = SU_td[ST[hit[i]]]; B[i] = np.clip(_bary(np.r_[mapped[i]], a, b, c), 0, 1); B[i] /= B[i].sum()
    sampled = S[ST[hit, 0]] * B[:, :1] + S[ST[hit, 1]] * B[:, 1:2] + S[ST[hit, 2]] * B[:, 2:]
    return sampled, uv_dist <= miss_px / 1024, uv_dist, hit


# ---- masks from the Legacy donor itself --------------------------------------------------------------------
def legacy_masks(ex, policy=M.ATLAS_POLICY_NHL_HEAD_A, rings=2):
    """Structure-derived masks (attachment from body-weighted boundary records, rear opening, mouth loop, seams) plus
    palette-bucket masks (eyelids, lips, jaw) from the donor's own weights, plus atlas-band masks (lash strips, ear
    core) from the atlas policy, applicable only because the template identity was measured."""
    h = LX.submesh(ex, 'head'); st = h.get('structure') or LX.derive_head_structure(h)
    P, T, U = LX.head_arrays(ex); n = len(P); dup = [np.array(g) for g in st['seam_groups']]; adj = G.adjacency(n, T, dup)
    att = np.array(st['attachment_loop'], int); frozen = G.grow(att, adj, rings); is_frozen = np.zeros(n, bool); is_frozen[frozen] = True
    pal = LX.palettes_of(h['joints'], h['weights']); bucket = {}
    for name, ps in LX.PALETTE_BUCKETS.items():
        m = np.zeros(n, bool)
        for i, s in enumerate(pal):
            if any(p in ps for p in s): m[i] = True
        bucket[name] = m
    lb = policy['lash_band']; lash = ((U[:, 1] > lb['v'][0]) & (U[:, 1] < lb['v'][1])) & np.any([(U[:, 0] > u0) & (U[:, 0] < u1) for u0, u1 in lb['u']], axis=0)
    ec = policy['ear_core']; ear_core = np.any([(U[:, 0] > u0) & (U[:, 0] < u1) for u0, u1 in ec['u']], axis=0) & (U[:, 1] > ec['v'][0]) & (U[:, 1] < ec['v'][1]) & (np.abs(P[:, 0]) > ec['min_abs_x_m'])
    fw = policy['face_window']; face = M.in_rect(U, fw['u'], fw['v'])
    outer = FQ.outer_shell_faces(P, T); on_skin = np.zeros(n, bool); on_skin[np.unique(T[outer])] = True
    return {'P': P, 'T': T, 'U': U, 'dup': dup, 'adj': adj, 'attachment': att, 'frozen': frozen, 'is_frozen': is_frozen, 'rear_opening': np.array(st['rear_opening'], int), 'mouth_loop': np.array(st['mouth_opening'], int),
            'bucket': bucket, 'lash': lash, 'ear_core': ear_core, 'face_window': face, 'outer_faces': outer, 'on_skin': on_skin, 'structure': st}


def haircap_covered_mask(ex, mk, max_gap_m=0.03, poke_m=0.005):
    """Head records the native haircap covers: a ray along the record's outward normal hits the cap within max_gap
    (the cap is directly above: measured on player_67 the covered scalp lies 0-8 mm under the hair shell) or the cap
    lies within poke_m below the record (skin already through the shell). Rays are travel-limited so a face record
    whose inward ray crosses the skull and reaches the cap from inside does not count. Returns (mask, gap m per
    record: +travel to the cap above, -travel to the cap below, inf when neither)."""
    hc = LX.submesh(ex, 'haircap'); HP = G.native_to_blender(hc['positions']); HT = np.array(hc['triangles']); soup = G.TriangleSoup(HP, HT)
    P, T = mk['P'], mk['T']; vn = G.vertex_normals(P, T, mk['dup']); n = len(P); gap = np.full(n, np.inf); covered = np.zeros(n, bool)
    for i in range(n):
        up = soup.ray_hit(P[i], vn[i]); dn = soup.ray_hit(P[i], -vn[i])
        if up is not None and up < max_gap_m: covered[i] = True; gap[i] = up
        elif dn is not None and dn < poke_m: covered[i] = True; gap[i] = -dn
    return covered, gap


def haircap_rim_mask(ex, mk, tol_m=0.003, rings=1):
    """Head records where the native haircap's open rim meets the skin: within tol of any cap boundary record, grown
    by `rings`. Measured on player_67 run-04: reshaping these lifts the rim off the narrower CC5 nape/sideburns (the
    cap inner side becomes visible from the front and under the jaw), while the scalp interior under the hair volume
    is invisible. Returns (mask, distance m to the nearest rim record)."""
    hc = LX.submesh(ex, 'haircap'); HP = G.native_to_blender(hc['positions']); HT = np.array(hc['triangles']); rim = HP[G.open_boundary_vertices(HT)]
    P = mk['P']; d = np.full(len(P), np.inf)
    for s in range(0, len(P), 256): d[s:s + 256] = np.linalg.norm(P[s:s + 256, None, :] - rim[None, :, :], axis=2).min(1)
    ids = np.flatnonzero(d < tol_m); m = np.zeros(len(P), bool); m[G.grow(ids, mk['adj'], rings)] = True; return m, d


def socket_rim_sets(ex, mk, tol_mm=1.0):
    """Eye-socket rim records per side. The atlas lash band selects them (measured template identity); each record is
    assigned to the nearest DONOR cap centre and classified geometrically on the donor: 'on_cap' when its donor signed
    distance to that cap sphere is within tol (the lid margin resting on the eyeball), 'interior' otherwise (the
    socket-fan apex, ~10 mm deep on player_67). Also reports outer-shell non-band records within tol of a cap sphere and
    inside its angular extent (rim candidates the band would miss) so the classification is checkable, not assumed."""
    P = mk['P']; lash = mk['lash']; out = {'tol_mm': tol_mm, 'sides': {}, 'unbanded_on_cap': {}}
    caps = {}
    for role, side in (('eye_r', 'r'), ('eye_l', 'l')):
        cap = G.native_to_blender(LX.submesh(ex, role)['positions']); c0, r0, _ = sphere_fit(cap); u = (cap - c0) / np.linalg.norm(cap - c0, axis=1)[:, None]; ax = u.mean(0); ax /= np.linalg.norm(ax)
        caps[side] = (c0, r0, ax, float(np.degrees(np.arccos(np.clip(u @ ax, -1, 1))).max()))
    ids = np.flatnonzero(lash)
    if len(ids):
        dr = np.linalg.norm(P[ids] - caps['r'][0], axis=1); dl = np.linalg.norm(P[ids] - caps['l'][0], axis=1); side_of = np.where(dr <= dl, 'r', 'l')
    for side in ('r', 'l'):
        c0, r0, ax, half = caps[side]; mine = ids[side_of == side] if len(ids) else ids
        sd = (np.linalg.norm(P[mine] - c0, axis=1) - r0) * 1000 if len(mine) else np.zeros(0)
        on = mine[np.abs(sd) <= tol_mm]; interior = mine[np.abs(sd) > tol_mm]
        out['sides'][side] = {'ids': mine.tolist(), 'on_cap': on.tolist(), 'interior': interior.tolist(), 'donor_signed_mm': {int(i): float(v) for i, v in zip(mine, sd)}, 'centre': c0.tolist(), 'radius_m': float(r0), 'character_side': 'character right (native x < 0)' if side == 'r' else 'character left (native x > 0)'}
        cand = np.flatnonzero(mk['on_skin'] & ~lash & ~mk['is_frozen']); v = P[cand] - c0; d = np.linalg.norm(v, axis=1); ang = np.degrees(np.arccos(np.clip((v / np.maximum(d, 1e-12)[:, None]) @ ax, -1, 1)))
        near = cand[(np.abs(d - r0) * 1000 <= tol_mm) & (ang <= half)]; out['unbanded_on_cap'][side] = near.tolist()
    return out


def joint_smooth_socket(fitp, P, adj, on_skin, lash, is_frozen, rim_ids, c_placed, c_donor, r_cap, ax, half, rings=3, iters=80, floor_m=1e-3):
    """Bounded joint movement of the socket rim and its `rings` rows: a displacement field D over the region (boundary =
    the next row, D = 0) that is as smooth as possible (Jacobi Laplacian relaxation of D over the region) subject to
    (a) every rim record at exactly its donor distance from the placed cap centre and (b) every row record within the
    cap's angular extent at >= floor outside the placed cap. Constraints are re-projected radially after every
    relaxation pass, so the region moves as one smooth sheet instead of by per-record radial pushes (which fold small
    faces). Returns (new positions of the region, report)."""
    rim_ids = np.array(rim_ids, int); region = rim_ids.copy(); rows = []
    prev = rim_ids.copy(); seen = set(rim_ids.tolist())
    for k in range(rings):
        ring = np.setdiff1d(G.grow(prev, adj, 1), np.array(sorted(seen), int)); ring = ring[on_skin[ring] & ~lash[ring] & ~is_frozen[ring]]
        rows.append(ring); seen |= set(ring.tolist()); prev = np.union1d(prev, ring); region = np.union1d(region, ring)
    row_ids = np.concatenate(rows) if rows else np.zeros(0, int); is_rim = np.isin(region, rim_ids)
    D = np.zeros((len(fitp), 3)); X0 = fitp.copy(); reg_set = set(region.tolist())
    donor_r = {int(i): float(np.linalg.norm(P[i] - c_donor)) for i in rim_ids}
    def project(D):
        X = X0 + D; moved = 0.0
        for i in rim_ids:
            v = X[i] - c_placed; d = float(np.linalg.norm(v))
            if d > 1e-12: t = c_placed + v / d * donor_r[int(i)]; moved = max(moved, float(np.linalg.norm(t - X[i]))); D[i] = t - X0[i]
        for i in row_ids:
            v = X[i] - c_placed; d = float(np.linalg.norm(v)); ang = np.degrees(np.arccos(np.clip((v / max(d, 1e-12)) @ ax, -1, 1)))
            if ang <= half and d < r_cap + floor_m: t = c_placed + v / d * (r_cap + floor_m); moved = max(moved, float(np.linalg.norm(t - X[i]))); D[i] = t - X0[i]
        return D, moved
    D, _ = project(D); hist = []
    for it in range(iters):
        Dn = D.copy()
        for i in region:
            nb = [j for j in adj[i]]
            if nb: Dn[i] = np.mean([D[j] for j in nb], axis=0)        # neighbours outside the region carry D = 0 (fixed boundary)
        Dn, moved = project(Dn); delta = float(np.abs(Dn - D)[region].max()); D = Dn; hist.append(round(delta * 1000, 4))
        if delta < 1e-7: break
    X = X0 + D
    rep = {'region_records': int(len(region)), 'rim_records': int(len(rim_ids)), 'row_records': [int(len(r)) for r in rows], 'iterations': len(hist), 'last_delta_mm': hist[-1] if hist else 0.0, 'max_move_mm': float(np.linalg.norm(D[region], axis=1).max() * 1000) if len(region) else 0.0,
           'rim_radius_residual_mm': float(max(abs(np.linalg.norm(X[i] - c_placed) - donor_r[int(i)]) for i in rim_ids) * 1000) if len(rim_ids) else 0.0}
    return X, region, rep


def constrained_socket_refit(fitp, P, T, adj, dup, on_skin, lash, is_frozen, rim_ids, c_placed, c_donor, r_cap, ax, half, p, rim_target_shift_mm=0.0):
    """Bounded, fold-aware eye-region refit for one socket. Unknowns: a displacement D of the region records (rim + K rows;
    the next row is the fixed boundary, D = 0). One energy in millimetres, minimised by L-BFGS with a finite budget:
        w_smooth * |L D|^2                     umbrella Laplacian of the DISPLACEMENT (tangential shape and the fitted
                                               likeness are kept: only smooth deviations from the fit are allowed)
      + w_rim    * sum_rim (|X - c| - r_donor)^2       each rim record at its DONOR distance from the PLACED cap centre
      + w_floor  * sum_rows max(0, r_cap + floor - |X - c|)^2   rows inside the cap's angular extent stay >= floor outside it
      + w_orient * sum_faces max(0, m - n.n_ref)^2     no face rotates past the declared dot from its reference normal
                                               (reference = the fitted normal when it agrees with the donor's, else the
                                               donor's: the donor's inward anatomy stays allowed, folds are not)
      + w_area   * sum_faces max(0, a*A_ref - A)^2     no face collapses below a of its reference area
      + w_edge   * sum_edges (|e| - |e_fit|)^2         edge lengths near the fit's (tangential shape of the fit kept)
    Self-intersections are NOT in the energy: they are a stopping / qualification screen run by the caller on predicted
    native coordinates. `r_cap` is the (possibly declared-scaled) placed cap radius; `rim_target_shift_mm` = r_cap - r_donor_cap so
    every rim record keeps its DONOR SIGNED DISTANCE to the cap surface (0 for the unchanged cap). Returns (positions, region
    ids, report with residuals, iterations and the stopping reason)."""
    rim_ids = np.array(rim_ids, int); prev = rim_ids.copy(); seen = set(rim_ids.tolist()); rows = []; K = int(p['socket_con_rings']); floor = p['socket_con_floor_mm']
    def row_after(prev):
        ring = np.setdiff1d(G.grow(prev, adj, 1), np.array(sorted(seen), int)); return ring[on_skin[ring] & ~lash[ring] & ~is_frozen[ring]]
    def in_extent(X, ids):
        v = X[ids] - c_placed; d = np.linalg.norm(v, axis=1); ang = np.degrees(np.arccos(np.clip((v / np.maximum(d, 1e-12)[:, None]) @ ax, -1, 1))); return (ang <= half), d
    support = {'rule': 'declared K' if K else f'grow rows until every row record within the cap extent is >= {p["socket_con_support_clear_mm"]} mm outside the placed cap, then one boundary row', 'rows': []}
    k = 0
    while True:
        ring = row_after(prev)
        if not len(ring): break
        ext, d = in_extent(fitp, ring); clear = float(((d - r_cap) * 1000)[ext].min()) if ext.any() else float('inf')
        support['rows'].append({'row': k + 1, 'records': int(len(ring)), 'min_clearance_in_extent_mm': clear}); rows.append(ring); seen |= set(ring.tolist()); prev = np.union1d(prev, ring); k += 1
        if K and k >= K: break
        if not K and clear >= p['socket_con_support_clear_mm']: rows.append(row_after(prev)); seen |= set(rows[-1].tolist()); support['rows'].append({'row': k + 1, 'records': int(len(rows[-1])), 'boundary_row': True}); break
        if k >= 12: break
    region = np.concatenate([rim_ids] + rows) if rows else rim_ids.copy(); support['K'] = len(rows)
    # seam closure: a seam group with one member in the region brings its twins in (non-frozen), so tied records are solved as one
    # variable (gradients averaged within the group; equal starts stay equal under every L-BFGS step) instead of being averaged afterwards
    tied = [np.asarray(g_, int) for g_ in dup if np.isin(g_, region).any()]; extra = [int(i) for g_ in tied for i in g_ if int(i) not in set(region.tolist()) and not is_frozen[int(i)]]
    if extra: region = np.concatenate([region, np.array(sorted(set(extra)), int)]); support['seam_closure_records'] = len(set(extra))
    nR = len(region); loc = {int(i): j for j, i in enumerate(region)}
    is_rim = np.isin(region, rim_ids); row_loc = np.flatnonzero(~is_rim); rim_loc = np.flatnonzero(is_rim)
    # geometry in mm
    X0 = fitp[region] * 1000; C = c_placed * 1000; RC = r_cap * 1000; donor_r = np.linalg.norm(P[rim_ids] - c_donor, axis=1) * 1000 + float(rim_target_shift_mm)
    # smoothness operator L = I - W over the region (neighbours outside carry D = 0)
    L = np.eye(nR)
    for j, i in enumerate(region):
        nb = list(adj[i]); w = 1.0 / max(len(nb), 1)
        for q in nb:
            if int(q) in loc: L[j, loc[int(q)]] -= w
    LtL = L.T @ L
    # faces touching the region (all three positions needed; outside records fixed at the fit)
    in_reg = np.zeros(len(P), bool); in_reg[region] = True; faces = np.flatnonzero(in_reg[T].any(1)); FT = T[faces]      # EVERY face touching a region record (outer, fold underside, fan): a face whose records are occluded along the donor normal still folds and is still seen (crease-02: face 4770)
    full0 = fitp * 1000; nd = FQ.face_normals(P * 1000, FT); nf0 = FQ.face_normals(full0, FT); agree = (nd * nf0).sum(1) >= p['socket_con_orient_min_dot']; n_ref = np.where(agree[:, None], nf0, nd)
    A_ref = 0.5 * np.linalg.norm(np.cross(full0[FT[:, 1]] - full0[FT[:, 0]], full0[FT[:, 2]] - full0[FT[:, 0]]), axis=1); A_don = 0.5 * np.linalg.norm(np.cross(P[FT[:, 1]] - P[FT[:, 0]], P[FT[:, 2]] - P[FT[:, 0]]), axis=1) * 1e6; A_ref = np.minimum(A_ref, A_don)
    edges = np.unique(np.sort(np.concatenate([FT[:, [0, 1]], FT[:, [1, 2]], FT[:, [2, 0]]]), axis=1), axis=0); edges = edges[in_reg[edges].any(1)]; l_don = np.linalg.norm(P[edges[:, 0]] - P[edges[:, 1]], axis=1) * 1000; l_fit = np.linalg.norm(full0[edges[:, 0]] - full0[edges[:, 1]], axis=1)   # edge reference = the FITTED lengths (the fit legitimately reshapes the donor; the term keeps the fit's tangential shape); the donor ratio is reported
    fl = np.array([[loc.get(int(v), -1) for v in f] for f in FT]); el = np.array([[loc.get(int(v), -1) for v in e] for e in edges])
    m_o = p['socket_con_orient_min_dot']; a_r = p['socket_con_area_ratio']; W = {k_: p[f'socket_con_w_{k_}'] for k_ in ('smooth', 'rim', 'floor', 'orient', 'area', 'edge', 'crease')}
    # coupled crease term: interior edges of the support faces + their surround, envelope from the donor and the fit input
    ES = CR.edge_structure(T, faces, None); th_d = CR.signed_dihedrals(P * 1000, T, ES); th_0 = CR.signed_dihedrals(full0, T, ES)
    if p.get('socket_con_crease_reference', 'donor_fit_input') not in CREASE_REFERENCES: raise ValueError(f"socket_con_crease_reference {p.get('socket_con_crease_reference')} unsupported {CREASE_REFERENCES}")
    if p.get('socket_con_crease_reference', 'donor_fit_input') == 'donor_signed': cr_lo, cr_hi, cr_is_crease = CR.envelope_donor_signed(th_d, np.radians(p['socket_con_crease_margin_deg']), np.radians(p.get('socket_con_crease_sign_deg', 15.0)), None if p.get('socket_con_crease_free_deg') is None else np.radians(p['socket_con_crease_free_deg']))
    else: cr_lo, cr_hi = CR.envelope([th_d, th_0], np.radians(p['socket_con_crease_margin_deg'])); cr_is_crease = np.abs(th_d) >= np.radians(p.get('socket_con_crease_sign_deg', 15.0))      # support faces' interior edges + the surround (every adjacent face), no visibility filter
    cr_cls = CR.classify_edges(ES, th_d, lash[T].all(1), on_skin[T].all(1)); rec_in_edges = np.unique(np.concatenate([ES['a'], ES['b'], ES['c'], ES['d']])) if ES['n'] else np.zeros(0, int)
    def scatter(grad_full, rows_idx, g):
        for col in range(rows_idx.shape[1]):
            m = rows_idx[:, col] >= 0; np.add.at(grad_full, rows_idx[m, col], g[col][m])
    def energy(Dflat):
        D = Dflat.reshape(nR, 3); X = X0 + D; full = full0.copy(); full[region] = X; E = 0.0; g = np.zeros((nR, 3)); terms = {}
        LD = L @ D; terms['smooth'] = float((LD ** 2).sum()); g += 2 * LtL @ D * W['smooth']
        v = X[rim_loc] - C; d = np.linalg.norm(v, axis=1); res = d - donor_r; terms['rim'] = float((res ** 2).sum()); g[rim_loc] += (2 * res / np.maximum(d, 1e-9))[:, None] * v * W['rim']
        v = X[row_loc] - C; d = np.linalg.norm(v, axis=1); ang = np.degrees(np.arccos(np.clip((v / np.maximum(d, 1e-9)[:, None]) @ ax, -1, 1))); res = np.where(ang <= half, np.maximum(0, RC + floor - d), 0); terms['floor'] = float((res ** 2).sum()); g[row_loc] += (-2 * res / np.maximum(d, 1e-9))[:, None] * v * W['floor']
        a, b, c = full[FT[:, 0]], full[FT[:, 1]], full[FT[:, 2]]; N = np.cross(b - a, c - a); nl = np.linalg.norm(N, axis=1); nh = N / np.maximum(nl, 1e-12)[:, None]; sdot = (nh * n_ref).sum(1); res = np.maximum(0, m_o - sdot); terms['orient'] = float((res ** 2).sum())
        gg = (n_ref - sdot[:, None] * nh) / np.maximum(nl, 1e-12)[:, None]; coef = (-2 * res)[:, None]
        scatter(g, fl, [coef * np.cross(b - c, gg) * W['orient'], coef * np.cross(c - a, gg) * W['orient'], coef * np.cross(a - b, gg) * W['orient']])
        A = 0.5 * nl; res = np.maximum(0, a_r * A_ref - A); terms['area'] = float((res ** 2).sum()); coef = (-2 * res)[:, None] * 0.5
        scatter(g, fl, [coef * np.cross(b - c, nh) * W['area'], coef * np.cross(c - a, nh) * W['area'], coef * np.cross(a - b, nh) * W['area']])
        e = full[edges[:, 0]] - full[edges[:, 1]]; l = np.linalg.norm(e, axis=1); res = l - l_fit; terms['edge'] = float((res ** 2).sum()); ge = (2 * res / np.maximum(l, 1e-9))[:, None] * e * W['edge']
        scatter(g, el, [ge, -ge])
        if W['crease'] > 0:
            Ec, gc, _ = CR.crease_energy(full, T, ES, cr_lo, cr_hi); terms['crease'] = float(Ec); g += gc[region] * W['crease']
        else: terms['crease'] = 0.0
        E = sum(W[k_] * terms[k_] for k_ in terms)
        for grp in tied:                      # seam duplicates inside the region move together
            jj = [loc[int(i)] for i in grp if int(i) in loc]; g[jj] = g[jj].mean(0)
        return E, g.ravel(), terms
    # L-BFGS (two-loop recursion) with Armijo backtracking; budget = iters / tol; stopping reason recorded
    x = np.zeros(nR * 3) if p['socket_con_init'] is None else (np.asarray(p['socket_con_init'], float)[region] * 1000 - X0).ravel()
    E, g, terms = energy(x); hist_s, hist_y = [], []; reason = 'iteration budget'; it = 0; E0 = E; line_fail = 0; small_steps = 0; slow = 0; gradcheck = None
    if p.get('socket_con_gradcheck'):
        # finite-difference check of the analytic gradient at a perturbed start (all terms active), recorded in the report
        # categories: random coords; coords of records on ACTIVE hinges (floor / orientation / crease residual > 0); records of the
        # smallest-area faces; tied seam records - at two points: the start (constraints violated) and a perturbed point
        rng = np.random.default_rng(0); h = 1e-4; gradcheck = {'points': {}}
        def active_records(Dflat):
            D = Dflat.reshape(nR, 3); X = X0 + D; full = full0.copy(); full[region] = X; ids = set()
            v = X[row_loc] - C; d = np.linalg.norm(v, axis=1); ang = np.degrees(np.arccos(np.clip((v / np.maximum(d, 1e-9)[:, None]) @ ax, -1, 1))); ids |= set(region[row_loc[(ang <= half) & (d < RC + floor)]].tolist())
            nh = FQ.face_normals(full, FT); ids |= set(FT[(nh * n_ref).sum(1) < m_o].ravel().tolist())
            if ES['n']:
                th = CR.wrap_to_envelope(CR.signed_dihedrals(full, T, ES), cr_lo, cr_hi); bad = (th > cr_hi) | (th < cr_lo); ids |= set(np.concatenate([ES['a'][bad], ES['b'][bad], ES['c'][bad], ES['d'][bad]]).tolist())
            return np.array([loc[i] for i in ids if i in loc], int)
        small = np.array([loc[int(v)] for f_ in np.argsort(A_ref)[:5] for v in FT[f_] if int(v) in loc], int); seam = np.array([loc[int(i)] for grp in tied for i in grp if int(i) in loc], int)
        for pname, xp in (('start', x.copy()), ('perturbed', x + rng.normal(0, 0.3, nR * 3))):
            Ep, gp, _ = energy(xp); cats = {'random': rng.choice(nR * 3, size=min(8, nR * 3), replace=False), 'active_hinges': (active_records(xp)[:, None] * 3 + np.arange(3)).ravel(), 'small_area_faces': (small[:, None] * 3 + np.arange(3)).ravel() if len(small) else np.zeros(0, int), 'seam_tied': (seam[:, None] * 3 + np.arange(3)).ravel() if len(seam) else np.zeros(0, int)}
            res = {}
            for cname, coords in cats.items():
                coords = np.unique(coords)[:12]; errs = []
                for j in coords:
                    e1 = np.zeros(nR * 3); e1[j] = h; gj = float(gp[j])
                    if cname == 'seam_tied':          # tied twins move together: perturb the whole group, compare with the summed (= group size x averaged) gradient
                        grp = [q_ for grp_ in tied for q_ in [loc[int(i)] for i in grp_ if int(i) in loc] if j // 3 in [loc[int(i)] for i in grp_ if int(i) in loc]]
                        for q_ in grp: e1[q_ * 3 + j % 3] = h
                        gj = float(sum(gp[q_ * 3 + j % 3] for q_ in grp))
                    fd = (energy(xp + e1)[0] - energy(xp - e1)[0]) / (2 * h); errs.append((float(fd), gj))
                res[cname] = {'coords': int(len(errs)), 'max_abs_error': float(max((abs(a_ - b_) for a_, b_ in errs), default=0.0)), 'max_rel_error': float(max((abs(a_ - b_) / max(abs(a_), abs(b_), 1e-9) for a_, b_ in errs), default=0.0))}
            gradcheck['points'][pname] = res
        gradcheck['max_rel_error'] = max(v['max_rel_error'] for pt in gradcheck['points'].values() for v in pt.values()); gradcheck['max_abs_error'] = max(v['max_abs_error'] for pt in gradcheck['points'].values() for v in pt.values())
        # step-size / directional study (review 2026-09-16): the earlier small-face START-point discrepancies (abs up to 8e-3 with |g| ~ 1e2-1e4) are either FD
        # truncation on a kinked hinge or a gradient defect. Central differences at h = 1e-3 .. 1e-6 on the small-face coords, plus the directional
        # derivative along the analytic gradient direction (FD of E along u = g/|g| vs g . u): a truncation error shrinks with h (until roundoff), a defect does not.
        Es, gs, _ = energy(x); un = gs / max(np.linalg.norm(gs), 1e-30); study = {'small_area_face_coords': {}, 'directional_along_gradient': {}, 'analytic_directional': float(gs @ un)}
        for hh in (1e-3, 1e-4, 1e-5, 1e-6):
            errs = []
            for j in (np.unique(small)[:4] if len(small) else []):
                for ax_ in range(3):
                    e1 = np.zeros(nR * 3); e1[j * 3 + ax_] = hh; fd = (energy(x + e1)[0] - energy(x - e1)[0]) / (2 * hh); errs.append(abs(fd - float(gs[j * 3 + ax_])))
            study['small_area_face_coords'][f'{hh:.0e}'] = {'coords': len(errs), 'max_abs_error': float(max(errs, default=0.0))}
            fdd = (energy(x + hh * un)[0] - energy(x - hh * un)[0]) / (2 * hh); study['directional_along_gradient'][f'{hh:.0e}'] = {'fd': float(fdd), 'abs_error': float(abs(fdd - gs @ un)), 'rel_error': float(abs(fdd - gs @ un) / max(abs(gs @ un), 1e-12))}
        gradcheck['step_size_study'] = study
    for it in range(1, int(p['socket_con_iters']) + 1):
        q = g.copy(); al = []
        for s_, y_ in reversed(list(zip(hist_s, hist_y))): a_ = (s_ @ q) / max(y_ @ s_, 1e-20); al.append(a_); q -= a_ * y_
        if hist_s: q *= (hist_s[-1] @ hist_y[-1]) / max(hist_y[-1] @ hist_y[-1], 1e-20)
        else: q *= 0.05 / max(np.abs(g).max(), 1e-12)
        for (s_, y_), a_ in zip(zip(hist_s, hist_y), reversed(al)): q += s_ * (a_ - (y_ @ q) / max(y_ @ s_, 1e-20))
        dvec = -q; slope = g @ dvec
        if slope >= 0: dvec = -g; slope = g @ dvec; hist_s, hist_y = [], []
        t = 1.0; ok = False
        for _ in range(30):
            xn = x + t * dvec; En, gn, tn = energy(xn)
            if En <= E + 1e-4 * t * slope: ok = True; break
            t *= 0.5
        if not ok:
            # a failed search along the quasi-Newton direction is not convergence with hinge terms: restart from steepest descent once
            if hist_s: hist_s, hist_y = [], []; line_fail += 1; continue
            reason = 'line search failed along steepest descent (energy stationary at the tolerance of the search)'; break
        s_ = xn - x; y_ = gn - g
        if y_ @ s_ > 1e-16: hist_s.append(s_); hist_y.append(y_); hist_s = hist_s[-12:]; hist_y = hist_y[-12:]
        rel = (E - En) / max(abs(E), 1e-20); x, E, g, terms = xn, En, gn, tn
        if t < 1e-3: hist_s, hist_y = [], []; small_steps += 1; slow = 0; continue        # a tiny accepted step (kinked hinge): reset the memory, never count it toward convergence
        slow = slow + 1 if rel < p['socket_con_tol'] else 0
        if slow >= 3: reason = f'relative energy decrease below tol {p["socket_con_tol"]} on 3 consecutive full steps'; break
        if np.abs(g).max() < 1e-9: reason = 'gradient below 1e-9'; break
    D = x.reshape(nR, 3); X = X0 + D; full = full0.copy(); full[region] = X
    for grp in tied:
        jj = [loc[int(i)] for i in grp if int(i) in loc]; X[jj] = X[jj].mean(0)
    lattice = None
    if p['socket_con_lattice']:
        from . import legacy_quant as LQt
        snap = lambda Xmm: G.native_to_blender(LQt.native_round(G.blender_to_native(np.asarray(Xmm) / 1000))) * 1000
        E_cont = energy(x)[0]; full0_q = snap(full0); full0_bak = full0.copy(); full0[:] = full0_q          # the surround is stored too: evaluate against its stored values
        Xq = snap(X); xq = (Xq - X0).ravel(); E_q0 = energy(xq)[0]; ulp = LQt.half_ulp(G.blender_to_native(X / 1000)) * 1000   # per-record, per-native-axis step (mm)
        ulp_b = np.abs(G.native_to_blender(ulp / 1000)) * 1000                                                      # native (x, y_up, z) -> Blender (x, z, y) axes
        order = np.argsort(-ulp_b.max(1)); passes = 0; changed_total = 0; hist = [float(E_q0)]
        for passes in range(1, int(p['socket_con_lattice_passes']) + 1):
            changed = 0
            for j in order:
                base = xq.reshape(nR, 3)[j].copy(); best = (energy(xq)[0], None)
                for ax_ in range(3):
                    for k_ in (-2, -1, 1, 2):
                        trial = xq.copy(); trial[j * 3 + ax_] = base[ax_] + k_ * ulp_b[j, ax_]
                        trial[j * 3:j * 3 + 3] = snap((X0[j] + trial[j * 3:j * 3 + 3])[None])[0] - X0[j]      # every trial value is a stored value (a step across a binade boundary changes the step size)
                        Et = energy(trial)[0]
                        if Et < best[0] - 1e-12: best = (Et, trial)
                if best[1] is not None: xq = best[1]; changed += 1
            hist.append(float(energy(xq)[0])); changed_total += changed
            if changed == 0: break
        Xq = X0 + xq.reshape(nR, 3); drift = float(np.abs(snap(Xq) - Xq).max()); Xq = snap(Xq)                          # chosen values are stored values; the re-snap drift is reported (float rounding only)
        lattice = {'energy_continuous': float(E_cont), 'energy_snapped': float(E_q0), 'energy_lattice': float(energy((Xq - X0).ravel())[0]), 'passes': passes, 'records_changed': int(changed_total), 'energy_by_pass': hist, 'resnap_drift_mm': drift, 'vertical_step_mm': {'min': float(ulp_b[:, 2].min()), 'max': float(ulp_b[:, 2].max())}, 'note': 'surround evaluated at its stored values; coordinate descent over floor/ceil/+-1 step per axis of the stored grid'}
        X = Xq; full = full0.copy(); full[region] = X; D = X - X0; x = D.ravel(); E, g, terms = energy(x)
    v = X[rim_loc] - C; rim_res = np.abs(np.linalg.norm(v, axis=1) - donor_r); v = X[row_loc] - C; d = np.linalg.norm(v, axis=1)
    ang = np.degrees(np.arccos(np.clip((v / np.maximum(d, 1e-9)[:, None]) @ ax, -1, 1))); fl_v = np.where(ang <= half, np.maximum(0, RC + floor - d), 0)
    a, b, c = full[FT[:, 0]], full[FT[:, 1]], full[FT[:, 2]]; N = np.cross(b - a, c - a); nl = np.linalg.norm(N, axis=1); nh = N / np.maximum(nl, 1e-12)[:, None]; sdot = (nh * n_ref).sum(1); A = 0.5 * nl
    ed = np.linalg.norm(full[edges[:, 0]] - full[edges[:, 1]], axis=1) / np.maximum(l_don, 1e-9); ef = np.linalg.norm(full[edges[:, 0]] - full[edges[:, 1]], axis=1) / np.maximum(l_fit, 1e-9)
    th_end = CR.wrap_to_envelope(CR.signed_dihedrals(full, T, ES), cr_lo, cr_hi) if ES['n'] else np.zeros(0); over = np.maximum(0, th_end - cr_hi) + np.minimum(0, th_end - cr_lo)      # wrapped like the energy
    th_end_raw = CR.signed_dihedrals(full, T, ES) if ES['n'] else np.zeros(0); rev_end = CR.sign_reversals(th_end_raw, th_d, cr_is_crease) if ES['n'] else np.zeros(0, bool); rev_0 = CR.sign_reversals(th_0, th_d, cr_is_crease) if ES['n'] else np.zeros(0, bool)
    crease_rep = {'edges': int(ES['n']), 'margin_deg': p['socket_con_crease_margin_deg'], 'reference_policy': p.get('socket_con_crease_reference', 'donor_fit_input'), 'sign_deg': p.get('socket_con_crease_sign_deg', 15.0), 'free_deg': p.get('socket_con_crease_free_deg'), 'crease_edges_(|donor| >= sign_deg)': int(cr_is_crease.sum()) if ES['n'] else 0, 'sign_reversals_vs_donor': {'fit_input': int(rev_0.sum()), 'end': int(rev_end.sum()), 'end_edges': [[int(ES['a'][i]), int(ES['b'][i])] for i in np.flatnonzero(rev_end)[:40]]},
                  'references': 'donor + fit input (per-edge envelope [min, max] +- margin)' if p.get('socket_con_crease_reference', 'donor_fit_input') == 'donor_fit_input' else 'donor +- margin, sign kept on crease edges (envelope_donor_signed)', 'classes': {k: int((cr_cls == k).sum()) for k in ('smooth', 'inherited_crease', 'fan', 'interior')},
                  'outside_envelope_at_start': int(((CR.wrap_to_envelope(th_0, cr_lo, cr_hi) > cr_hi) | (CR.wrap_to_envelope(th_0, cr_lo, cr_hi) < cr_lo)).sum()), 'outside_envelope_at_end': int((over != 0).sum()), 'max_excess_deg_at_end': float(np.degrees(np.abs(over).max())) if len(over) else 0.0,
                  'edges_outside_at_end': [{'a': int(ES['a'][i]), 'b': int(ES['b'][i]), 'faces': [int(ES['f'][i]), int(ES['g'][i])], 'class': str(cr_cls[i]), 'theta_deg': {'donor': float(np.degrees(th_d[i])), 'fit_input': float(np.degrees(th_0[i])), 'end': float(np.degrees(th_end[i]))}, 'excess_deg': float(np.degrees(over[i]))} for i in np.flatnonzero(over != 0)[:40]]}
    rep = {'gradcheck': gradcheck, 'lattice': lattice, 'crease': crease_rep, 'support': support, 'region_records': int(nR), 'rim_records': int(len(rim_ids)), 'row_records': int(len(row_loc)), 'faces': int(len(faces)), 'edges': int(len(edges)), 'tied_seam_groups': len(tied), 'weights': W, 'init': 'fit input' if p['socket_con_init'] is None else 'declared warm start', 'floor_mm': floor, 'orient_min_dot': m_o, 'area_ratio': a_r,
           'iterations': it, 'stopping_reason': reason, 'line_search': {'rule': 'Armijo 1e-4, up to 30 halvings from t = 1; failed quasi-Newton search -> memory reset and steepest descent; accepted step t < 1e-3 -> memory reset, not counted toward convergence; convergence = relative decrease < tol on 3 consecutive full steps', 'memory_resets': line_fail + small_steps, 'quasi_newton_failures': line_fail, 'tiny_steps': small_steps}, 'energy_start': float(E0), 'energy_end': float(E), 'terms_end': terms, 'max_move_mm': float(np.linalg.norm(D, axis=1).max()) if nR else 0.0, 'moved_over_0p1mm': int((np.linalg.norm(D, axis=1) > 0.1).sum()),
           'residuals': {'rim_radius_max_mm': float(rim_res.max()) if len(rim_res) else 0.0, 'rim_radius_median_mm': float(np.median(rim_res)) if len(rim_res) else 0.0, 'floor_violations': int((fl_v > 1e-6).sum()), 'floor_violation_max_mm': float(fl_v.max()) if len(fl_v) else 0.0, 'orient_min_dot_reached': float(sdot.min()), 'faces_below_orient_min': [int(faces[i]) for i in np.flatnonzero(sdot < m_o - 1e-6)],
                         'area_min_ratio_reached': float((A / np.maximum(A_ref, 1e-9)).min()), 'faces_below_area_ratio': [int(faces[i]) for i in np.flatnonzero(A < a_r * A_ref - 1e-9)], 'edge_ratio_vs_donor': {'min': float(ed.min()), 'max': float(ed.max())}, 'edge_ratio_vs_fit': {'min': float(ef.min()), 'max': float(ef.max())}, 'reference_normals_from_donor': int((~agree).sum())}}
    return full / 1000, region, rep


# ---- fit ------------------------------------------------------------------------------------------------------
def fit(ex, src, registration, shift=None, **params):
    """registration: {'kind': 'atlas_a', 'pairs_pixels', 'coefficients'} | {'kind': 'legacy_surface', 'anchors', 'coefficients'} | {'kind': 'surface'}.
    Returns (fit positions Blender, mode array, report, masks)."""
    p = {**DEFAULTS, **params}; t0 = time.time(); mk = legacy_masks(ex, rings=p['rings']); P, T, U, adj, dup = mk['P'], mk['T'], mk['U'], mk['adj'], mk['dup']; n = len(P)
    S0, ST, SU = C.head_arrays(src); SU_td = np.c_[SU[:, 0], 1 - SU[:, 1]]
    SE, _ = eye_centres_cc5(src); DE, _ = eye_centres_legacy(ex)
    shift = align_shift(SE, DE) if shift is None else np.asarray(shift, float); S = S0 + shift; CAPC = placed_cap_centres(SE, shift, p['cap_centre_override']); CAPS = cap_radius_scales(p)
    if (CAPS != 1).any() and p['socket_rim_policy'] in ('joint_smooth', 'donor_block'): raise ValueError(f"cap_radius_scale is not supported with socket_rim_policy {p['socket_rim_policy']} (never generalised to a scaled cap)")
    is_frozen = mk['is_frozen']; frozen = mk['frozen']; lash = mk['lash']; ear_core = mk['ear_core']; on_skin = mk['on_skin']
    band_top = float(P[mk['attachment'], 2].max() + p['band_height']); below = (P[:, 2] < band_top) & ~is_frozen
    mode = np.full(n, 'uv', object); D = np.zeros_like(P); reg_report = {}
    if registration['kind'] == 'surface':
        Q, dist, K, _ = surface_correspondence(P, S, ST, SU_td); D = Q - P; covered = np.ones(n, bool); mode[:] = 'surface'; uv_dist = np.zeros(n)
        reg_report = {'kind': 'surface', 'nearest_mm': {'median': float(np.median(dist) * 1000), 'p95': float(np.percentile(dist, 95) * 1000), 'max': float(dist.max() * 1000)}}
    else:
        if registration['kind'] == 'atlas_a':
            anchors = np.array([a for a, b in registration['pairs_pixels']], float) / 1024; coef = np.array(registration['coefficients'])
        else:
            anchors = np.asarray(registration['anchors'], float); coef = np.asarray(registration['coefficients'], float)
        mapped = F.tps_warp(anchors, coef, U); sampled, covered, uv_dist, hit = sample_by_uv(mapped, S, ST, SU_td, p['raster'], p['miss_px']); D = sampled - P
        reg_report = {'kind': registration['kind'], 'anchors': int(len(anchors)), 'uv_miss_px': {'max': float(uv_dist.max() * 1024), 'over_miss_px': int((~covered).sum())}}
    rear = (P[:, 1] > p['rear_y']) & ~np.isin(np.arange(n), G.grow(np.flatnonzero(ear_core), adj, 2)) & on_skin if registration['kind'] != 'surface' else np.zeros(n, bool)
    mode[~covered | rear] = 'uncovered'; mode[below] = 'band'; mode[is_frozen] = 'frozen'; D[is_frozen] = 0
    radial_hit = 0
    if registration['kind'] != 'surface':
        centre = np.array([0.0, float(S[S[:, 2] > DE[:, 2].mean(), 1].mean()), float(DE[:, 2].mean() + 0.02)]); soup = G.TriangleSoup(S, ST)
        for i in np.flatnonzero((mode == 'uncovered') & ~below):
            d = P[i] - centre; d /= np.linalg.norm(d); t = soup.ray_hit(centre, d)
            if t is None: continue
            D[i] = centre + d * t - P[i]; mode[i] = 'radial'; radial_hit += 1
        if p['rear_blend_rings']:
            near = np.zeros(n, bool); near[G.grow(np.flatnonzero(mode == 'radial'), adj, p['rear_blend_rings'])] = True; mode[np.flatnonzero(near & (mode == 'uv'))] = 'blend'
    scalp_report = None
    if p['scalp_policy'] not in SCALP_POLICIES: raise ValueError(f"scalp_policy {p['scalp_policy']} unsupported {SCALP_POLICIES}")
    if p['scalp_policy'] == 'preserve_haircap_rim':
        # declared route policy: only the skin the cap rim rests on keeps the donor positions (+1 ring); the scalp under
        # the hair volume follows the fit (invisible; poking through the shell is measured separately); blend N rings.
        rimm, _ = haircap_rim_mask(ex, mk, p['scalp_rim_tol_mm'] / 1000); rimm &= ~is_frozen
        if p['scalp_rim_below_eyes_mm'] is not None: rimm &= P[:, 2] <= DE[:, 2].mean() - p['scalp_rim_below_eyes_mm'] / 1000      # nape / lower-sideburn rim only: run-04 lifted the rim there; the temple and hairline rim overlaps the hair volume
        ids = np.flatnonzero(rimm)
        blend_ids = np.setdiff1d(G.grow(ids, adj, p['scalp_blend_rings']), ids); blend_ids = blend_ids[~is_frozen[blend_ids] & ~lash[blend_ids]]
        D[ids] = 0; mode[ids] = 'scalp_frozen'; mode[blend_ids] = np.where(mode[blend_ids] == 'band', 'band', 'scalp_blend')
        scalp_report = {'policy': 'preserve_haircap_rim', 'rim_records': int(len(ids)), 'blend_records': int(len(blend_ids)), 'blend_rings': p['scalp_blend_rings'], 'rim_tol_mm': p['scalp_rim_tol_mm'], 'below_eyes_mm': p['scalp_rim_below_eyes_mm']}
    if p['scalp_policy'] == 'preserve_under_haircap':
        # declared route policy: records the untouched native haircap covers keep their donor positions (the hair volume,
        # its rim contact with the skin and the rear-opening clearance stay exactly as the donor was built); a blend band of N
        # rings towards the fitted face (the visible hairline / temples / nape) is harmonic-filled between them.
        cov, gap_cap = haircap_covered_mask(ex, mk, p['scalp_max_gap_mm'] / 1000); cov &= ~is_frozen; ids = np.flatnonzero(cov)
        blend_ids = np.setdiff1d(G.grow(ids, adj, p['scalp_blend_rings']), ids); blend_ids = blend_ids[~is_frozen[blend_ids] & ~lash[blend_ids]]
        D[ids] = 0; mode[ids] = 'scalp_frozen'; mode[blend_ids] = np.where(mode[blend_ids] == 'band', 'band', 'scalp_blend')
        scalp_report = {'policy': 'preserve_under_haircap', 'covered_records': int(len(ids)), 'blend_records': int(len(blend_ids)), 'blend_rings': p['scalp_blend_rings'], 'max_gap_mm': p['scalp_max_gap_mm'], 'covered_gap_mm': {'median': float(np.median(gap_cap[ids]) * 1000), 'p95': float(np.percentile(gap_cap[ids], 95) * 1000)} if len(ids) else {}}
    free = np.flatnonzero((mode == 'uncovered') | (mode == 'band') | (mode == 'blend') | (mode == 'scalp_blend')); nb = [np.array(sorted(adj[i])) for i in range(n)]
    its = G.laplace_fill(D, free, nb) if len(free) else 0
    for _ in range(p['smooth_passes']):
        ids = np.flatnonzero((mode == 'radial') | (mode == 'uncovered') | (mode == 'blend') | (mode == 'scalp_blend')); Dn = D.copy()
        for i in ids: Dn[i] = 0.5 * D[i] + 0.5 * D[nb[i]].mean(0)
        D = Dn
    fitp = P + D
    skinT = T[mk['outer_faces'] & ~lash[T].any(1)]; sk = G.TriangleSoup(P, skinT)
    for i in np.flatnonzero(lash):
        q, d, k = sk.nearest_face(P[i]); t = skinT[k]; b = _bary(q, *P[t]); fitp[i] = P[i] + (fitp[t[0]] * b[0] + fitp[t[1]] * b[1] + fitp[t[2]] * b[2]) - q; mode[i] = 'lash'
    if p['mouth_gap_policy'] not in MOUTH_GAP_POLICIES: raise ValueError(f"mouth_gap_policy {p['mouth_gap_policy']} unsupported {MOUTH_GAP_POLICIES}")
    if p['mouth_gap_policy'] == 'preserve_donor_gap' and len(mk['mouth_loop']):
        ml = mk['mouth_loop']; c_fit = fitp[ml].mean(0); c_don = P[ml].mean(0); rigid = c_fit + (P[ml] - c_don); delta = rigid - fitp[ml]
        ring = G.grow(ml, adj, p['mouth_gap_rings']); ring = np.setdiff1d(ring, ml); ring = ring[~is_frozen[ring]]
        Dm = np.zeros_like(P); Dm[ml] = delta; free_m = ring; its_m = G.laplace_fill(Dm, free_m, nb) if len(free_m) else 0
        fitp = fitp + Dm; mode[ml] = 'mouth_rigid'
    lip_report = None
    if p['mouth_gap_policy'] == 'preserve_donor_lips' and len(mk['mouth_loop']):
        ml = mk['mouth_loop']; block = G.grow(ml, adj, p['lip_block_rings']); block = block[~is_frozen[block]]
        t = (fitp[block] - P[block]).mean(0); lip_report = {'policy': 'preserve_donor_lips', 'block_records': int(len(block)), 'requested_translation_m': t.tolist(), 'grid': p['lip_block_grid'], 'separation': p['lip_blend_separation']}
        if p['lip_block_grid'] not in LIP_BLOCK_GRIDS: raise ValueError(f"lip_block_grid {p['lip_block_grid']} unsupported {LIP_BLOCK_GRIDS}")
        if p['lip_block_grid'] == 'native_half':
            from . import legacy_quant as LQt
            tn, grep = LQt.grid_translation(G.blender_to_native(P[block]), G.blender_to_native(t[None])[0]); t = G.native_to_blender(tn[None])[0]
            lip_report['grid_translation'] = grep; lip_report['translation_m'] = t.tolist(); lip_report['block_storage_exact'] = bool(LQt.is_representable(G.blender_to_native(P[block] + t)).all())
        fitp[block] = P[block] + t; mode[block] = 'lip_block'
        outer_r = np.setdiff1d(G.grow(block, adj, p['lip_block_blend_rings']), block); outer_r = outer_r[~is_frozen[outer_r] & ~lash[outer_r]]
        if p['lip_block_blend_mode'] not in LIP_BLOCK_BLEND_MODES: raise ValueError(f"lip_block_blend_mode {p['lip_block_blend_mode']} unsupported {LIP_BLOCK_BLEND_MODES}")
        lip_report['blend_mode'] = p['lip_block_blend_mode']
        if p['lip_block_blend_mode'] == 'graded':
            R = p['lip_block_blend_rings']; prev = block; w_of = {}
            for k in range(1, R + 1):
                ring = np.setdiff1d(G.grow(prev, adj, 1), prev); prev = np.union1d(prev, ring)
                for i in ring:
                    if i in set(outer_r.tolist()): w_of[int(i)] = (R - k + 1) / (R + 1)
            for i, w in w_of.items(): fitp[i] = w * (P[i] + t) + (1 - w) * fitp[i]
            for ids in dup: fitp[ids] = fitp[ids].mean(0)
            lip_report['graded_weights_by_ring'] = {str(k): (R - k + 1) / (R + 1) for k in range(1, R + 1)}
        else:
            for _ in range(p['lip_block_blend_passes']):
                new = fitp.copy()
                for i in outer_r: new[i] = 0.5 * fitp[i] + 0.5 * fitp[nb[i]].mean(0)
                fitp = new
                for ids in dup: fitp[ids] = fitp[ids].mean(0)
        mode[outer_r] = np.where(mode[outer_r] == 'frozen', 'frozen', 'lip_block_blend')
        if p['lip_blend_separation'] not in LIP_BLEND_SEPARATIONS: raise ValueError(f"lip_blend_separation {p['lip_blend_separation']} unsupported {LIP_BLEND_SEPARATIONS}")
        if p['lip_blend_separation'] != 'none':
            # declared route policy: a blend-ring record that ended closer to the rigid block surface than its floor is pushed
            # straight away from the nearest block point to the floor (block faces = faces whose three records are in the block).
            from . import legacy_quant as LQt
            isb = np.zeros(n, bool); isb[block] = True; bfaces = np.flatnonzero(isb[T].all(1)); soup_d = G.TriangleSoup(P, T[bfaces]); pushed = {}
            bound = 0.5 * np.linalg.norm(LQt.half_ulp(G.blender_to_native(fitp[outer_r])), axis=1)
            for _ in range(p['lip_blend_separation_passes'] if p['lip_blend_separation'] in ('donor', 'storage') else 0):
                soup = G.TriangleSoup(fitp, T[bfaces]); moved_any = False
                for k, i in enumerate(outer_r):
                    if p['lip_blend_separation'] == 'donor': floor = soup_d.nearest(P[i])[1]
                    else: floor = 2.0 * bound[k]
                    q, d = soup.nearest(fitp[i])
                    if d < floor:
                        u = (fitp[i] - q) if d > 1e-9 else (P[i] - soup_d.nearest(P[i])[0]); u = u / max(np.linalg.norm(u), 1e-12)   # away from the block; the donor's own direction when the record sits on the surface
                        fitp[i] = q + u * floor; pushed[int(i)] = max(pushed.get(int(i), 0.0), float((floor - d) * 1000)); moved_any = True
                for ids in dup: fitp[ids] = fitp[ids].mean(0)
                if not moved_any: break
            if p['lip_blend_separation'] == 'donor_side':
                # plane-side restoration: for every blend record and every block face that is local to it in the donor
                # (closest point within `lip_side_radius_m`), the record must stay on the donor's side of that face's plane by
                # at least min(2 x its storage error bound, the donor's own distance). The block is a rigid translate of the
                # donor, so its face planes keep their donor normals; the push is along that normal only.
                fn_d = FQ.face_normals(P, T[bfaces]); pushed = {}; cons = []; cen = P[T[bfaces]].mean(1)
                for k, i in enumerate(outer_r):
                    near = np.flatnonzero(np.linalg.norm(cen - P[i], axis=1) < p['lip_side_radius_m'])
                    for fk in near:
                        f = bfaces[fk]; a = P[T[f, 0]]; sd = float((P[i] - a) @ fn_d[fk])
                        if abs(sd) < 1e-9: continue
                        cons.append((int(i), int(fk), np.sign(sd), min(2.0 * bound[k], abs(sd))))
                for _ in range(p['lip_blend_separation_passes']):
                    moved_any = False
                    for i, fk, s, floor in cons:
                        f = bfaces[fk]; a = fitp[T[f, 0]]; d = float((fitp[i] - a) @ fn_d[fk])
                        if s * d < floor:
                            fitp[i] = fitp[i] + fn_d[fk] * s * (floor - s * d); pushed[i] = pushed.get(i, 0.0) + float((floor - s * d) * 1000); moved_any = True
                    for ids in dup: fitp[ids] = fitp[ids].mean(0)
                    if not moved_any: break
                lip_report['side_constraints'] = len(cons); lip_report['side_radius_m'] = p['lip_side_radius_m']
                viol = sum(1 for i, fk, s, floor in cons if s * float((fitp[i] - fitp[T[bfaces[fk], 0]]) @ fn_d[fk]) < floor - 1e-9); lip_report['side_constraints_unmet_after_passes'] = int(viol)
            if p['lip_blend_separation'] == 'crossing_driven':
                fitp, xrep = LQt.repair_block_crossings(fitp, P, T, mk, block, outer_r, G.blender_to_native, G.native_to_blender, max_iters=p['lip_crossing_max_iters']); lip_report['crossing_driven'] = xrep; pushed = {int(k): v for k, v in xrep['pushed_mm'].items()}
            lip_report['separation_pushed'] = {'records': len(pushed), 'max_push_mm': max(pushed.values(), default=0.0), 'ids': sorted(pushed)}
            soup = G.TriangleSoup(fitp, T[bfaces]); lip_report['blend_to_block_min_mm'] = float(min(soup.nearest(fitp[i])[1] for i in outer_r) * 1000)
    if p['lip_relax_rings'] and p['lip_relax_passes'] and len(mk['mouth_loop']):
        # declared route policy: Laplacian relaxation of the lip band (rings 1..lip_relax_rings around the mouth loop, loop itself and
        # frozen records excluded; seam groups kept). Bounded: no source projection, no change outside the band.
        ml = mk['mouth_loop']; band_ids = np.setdiff1d(G.grow(ml, adj, p['lip_relax_rings']), ml); band_ids = band_ids[~is_frozen[band_ids]]
        for _ in range(p['lip_relax_passes']):
            new = fitp.copy()
            for i in band_ids: new[i] = (1 - p['lip_relax_alpha']) * fitp[i] + p['lip_relax_alpha'] * fitp[nb[i]].mean(0)
            fitp = new
            for ids in dup: fitp[ids] = fitp[ids].mean(0)
        mode[band_ids] = np.where(mode[band_ids] == 'frozen', 'frozen', 'lip_relaxed')
    lid_report = None
    if p['lid_clearance_mm'] is not None:
        # declared route policy: outer-shell eyelid-bucket records (lash strips excluded) that fall inside the PLACED 16 mm
        # Legacy cap within its angular extent are pushed radially from the cap centre out to the declared clearance.
        # Bounded local edit of head positions only; the cap keeps its native shape and the declared placement.
        if p['lid_clearance_placement'] not in LID_CLEARANCE_PLACEMENTS: raise ValueError(f"lid_clearance_placement {p['lid_clearance_placement']} unsupported {LID_CLEARANCE_PLACEMENTS}")
        lid_report = {'policy': 'preserve_lid_clearance', 'clearance_mm': p['lid_clearance_mm'], 'placement': p['lid_clearance_placement'], 'scope': p['lid_clearance_scope'], 'cap_radius_scale': CAPS.tolist(), 'moved': {}, 'max_push_mm': 0.0}
        socket_sets = socket_rim_sets(ex, mk, p['socket_rim_tol_mm']) if p['lid_clearance_scope'] == 'socket_rings' else None
        for role, cidx, side in (('eye_r', 0, 'r'), ('eye_l', 1, 'l')):
            cap = G.native_to_blender(LX.submesh(ex, role)['positions']); c0, r0, _ = sphere_fit(cap); c = CAPC[cidx] if p['lid_clearance_placement'] == 'cc5_centre' else c0; r0 = r0 * CAPS[cidx]      # declared cap scale: the clearance reads the scaled radius
            u = (cap - c0) / np.linalg.norm(cap - c0, axis=1)[:, None]; ax = u.mean(0); ax /= np.linalg.norm(ax); half = np.degrees(np.arccos(np.clip(u @ ax, -1, 1))).max()
            m = (mk['bucket'][f'upper_eyelid_{side}'] | mk['bucket'][f'lower_eyelid_{side}']) & on_skin & ~lash & ~is_frozen
            if p['lid_clearance_scope'] not in LID_CLEARANCE_SCOPES: raise ValueError(f"lid_clearance_scope {p['lid_clearance_scope']} unsupported {LID_CLEARANCE_SCOPES}")
            if p['lid_clearance_scope'] == 'socket_rings':
                # declared route policy: the rows around the socket rim, whatever palette weights them (player_67's upper-margin rows are eye_control-weighted, not eyelid-bucket)
                rim = np.array(socket_sets['sides'][side]['ids'], int); ring = np.zeros(n, bool)
                if len(rim): ring[G.grow(rim, adj, p['socket_rings'])] = True
                m = (m | (ring & on_skin & ~lash & ~is_frozen)); lid_report.setdefault('socket_ring_records', {})[role] = int((ring & on_skin & ~lash & ~is_frozen).sum())
            ids = np.flatnonzero(m)
            v = fitp[ids] - c; d = np.linalg.norm(v, axis=1); ang = np.degrees(np.arccos(np.clip((v / np.maximum(d, 1e-12)[:, None]) @ ax, -1, 1)))
            need = (ang <= half) & (d < r0 + p['lid_clearance_mm'] / 1000); moved = 0
            for j in np.flatnonzero(need):
                target = c + v[j] / d[j] * (r0 + p['lid_clearance_mm'] / 1000); push = float(np.linalg.norm(target - fitp[ids[j]]) * 1000); lid_report['max_push_mm'] = max(lid_report['max_push_mm'], push); fitp[ids[j]] = target; mode[ids[j]] = 'lid_cleared'; moved += 1
            lid_report['moved'][role] = moved
    socket_report = None
    if p['socket_rim_policy'] == 'joint_smooth':
        # declared route policy: the rim keeps its donor radial relation and the rows around it keep the lid clearance floor, solved
        # jointly as one smooth displacement sheet (see joint_smooth_socket); runs after every other head edit; both sides
        sets = socket_rim_sets(ex, mk, p['socket_rim_tol_mm']); socket_report = {'policy': 'joint_smooth', 'tol_mm': p['socket_rim_tol_mm'], 'placement': p['lid_clearance_placement'], 'rings': p['socket_joint_rings'], 'floor_mm': p['socket_joint_floor_mm'], 'sides': {}, 'unbanded_on_cap': sets['unbanded_on_cap']}
        for role, cidx, side in (('eye_r', 0, 'r'), ('eye_l', 1, 'l')):
            sd_ = sets['sides'][side]; c0 = np.array(sd_['centre']); r0 = sd_['radius_m']; c = CAPC[cidx] if p['lid_clearance_placement'] == 'cc5_centre' else c0
            cap = G.native_to_blender(LX.submesh(ex, role)['positions']); u = (cap - c0) / np.linalg.norm(cap - c0, axis=1)[:, None]; ax = u.mean(0); ax /= np.linalg.norm(ax); half = float(np.degrees(np.arccos(np.clip(u @ ax, -1, 1))).max())
            ids = np.array(sd_['ids'], int)
            if not len(ids): socket_report['sides'][side] = {'role': role, 'records': 0}; continue
            X, region, rep = joint_smooth_socket(fitp, P, adj, on_skin, lash, is_frozen, ids, c, c0, r0, ax, half, rings=p['socket_joint_rings'], iters=p['socket_joint_iters'], floor_m=p['socket_joint_floor_mm'] / 1000)
            fitp[region] = X[region]; mode[region] = np.where(np.isin(region, ids), 'socket_rim', 'socket_ring')
            socket_report['sides'][side] = {'role': role, 'character_side': sd_['character_side'], 'records': int(len(ids)), 'on_cap': len(sd_['on_cap']), 'interior': len(sd_['interior']), **rep}
    if p['socket_rim_policy'] == 'constrained':
        # declared route policy: bounded fold-aware refit of the socket rim + K rows as one constrained energy (see constrained_socket_refit);
        # runs after every other head edit; both sides; K from geometry unless declared
        sets = socket_rim_sets(ex, mk, p['socket_rim_tol_mm']); socket_report = {'policy': 'constrained', 'tol_mm': p['socket_rim_tol_mm'], 'placement': p['lid_clearance_placement'], 'sides': {}, 'unbanded_on_cap': sets['unbanded_on_cap']}
        for role, cidx, side in (('eye_r', 0, 'r'), ('eye_l', 1, 'l')):
            sd_ = sets['sides'][side]; c0 = np.array(sd_['centre']); r0 = sd_['radius_m']; c = CAPC[cidx] if p['lid_clearance_placement'] == 'cc5_centre' else c0
            cap = G.native_to_blender(LX.submesh(ex, role)['positions']); u = (cap - c0) / np.linalg.norm(cap - c0, axis=1)[:, None]; ax = u.mean(0); ax /= np.linalg.norm(ax); half = float(np.degrees(np.arccos(np.clip(u @ ax, -1, 1))).max())
            ids = np.array(sd_['ids'], int)
            if not len(ids): socket_report['sides'][side] = {'role': role, 'records': 0}; continue
            X, region, rep = constrained_socket_refit(fitp, P, T, adj, dup, on_skin, lash, is_frozen, ids, c, c0, r0 * CAPS[cidx], ax, half, p, rim_target_shift_mm=(r0 * (CAPS[cidx] - 1)) * 1000); rep['cap_radius_scale'] = float(CAPS[cidx]); rep['cap_radius_mm'] = float(r0 * CAPS[cidx] * 1000)
            fitp[region] = X[region]; mode[region] = np.where(np.isin(region, ids), 'socket_rim', 'socket_ring'); prev_region = socket_report.get('_region_prev')
            rep['region_overlap_with_other_side'] = int(np.isin(region, prev_region).sum()) if prev_region is not None else 0; socket_report['_region_prev'] = region      # the two sockets are solved in sequence: an overlap means the second solve re-moves the first's records (reported; zero on player_67)
            socket_report['sides'][side] = {'role': role, 'character_side': sd_['character_side'], 'records': int(len(ids)), 'on_cap': len(sd_['on_cap']), 'interior': len(sd_['interior']), **rep}
        socket_report.pop('_region_prev', None)
    if p['socket_rim_policy'] == 'donor_block':
        # declared route policy: the socket rim and its K rows are the donor's geometry translated by the placed cap's translation
        # (rim/cap relation exactly the donor's, no fold inside the block); rows K+1..K+B blend with graded weights (B-k+1)/(B+1)
        # between the translated donor position and the fitted one; runs after every other head edit; both sides
        sets = socket_rim_sets(ex, mk, p['socket_rim_tol_mm']); socket_report = {'policy': 'donor_block', 'block_rings': p['socket_block_rings'], 'blend_rings': p['socket_block_blend_rings'], 'placement': p['lid_clearance_placement'], 'sides': {}, 'unbanded_on_cap': sets['unbanded_on_cap']}
        for role, cidx, side in (('eye_r', 0, 'r'), ('eye_l', 1, 'l')):
            sd_ = sets['sides'][side]; c0 = np.array(sd_['centre']); c = CAPC[cidx] if p['lid_clearance_placement'] == 'cc5_centre' else c0; t = c - c0; ids = np.array(sd_['ids'], int)
            if not len(ids): socket_report['sides'][side] = {'role': role, 'records': 0}; continue
            block = ids.copy(); prev = ids.copy(); seen = set(ids.tolist()); K = int(p['socket_block_rings']); B = int(p['socket_block_blend_rings']); rows = []
            for k in range(K + B):
                ring = np.setdiff1d(G.grow(prev, adj, 1), np.array(sorted(seen), int)); ring = ring[on_skin[ring] & ~lash[ring] & ~is_frozen[ring]]; rows.append(ring); seen |= set(ring.tolist()); prev = np.union1d(prev, ring)
            blk = np.concatenate([ids] + rows[:K]) if K else ids; moved_block = float(np.linalg.norm(fitp[blk] - (P[blk] + t), axis=1).max() * 1000)
            fitp[blk] = P[blk] + t; mode[blk] = np.where(np.isin(blk, ids), 'socket_rim', 'socket_ring'); wts = {}
            for k, ring in enumerate(rows[K:], start=1):
                w = (B - k + 1) / (B + 1); fitp[ring] = w * (P[ring] + t) + (1 - w) * fitp[ring]; mode[ring] = 'socket_blend'; wts[k] = {'records': int(len(ring)), 'weight': w}
            socket_report['sides'][side] = {'role': role, 'character_side': sd_['character_side'], 'records': int(len(ids)), 'on_cap': len(sd_['on_cap']), 'interior': len(sd_['interior']), 'block_records': int(len(blk)), 'cap_translation_mm': (t * 1000).tolist(), 'block_max_move_from_fit_mm': moved_block, 'blend': wts}
    if p['socket_rim_policy'] not in SOCKET_RIM_POLICIES: raise ValueError(f"socket_rim_policy {p['socket_rim_policy']} unsupported {SOCKET_RIM_POLICIES}")
    if p['socket_rim_policy'] == 'preserve_donor_radius':
        # declared route policy (after every other head edit): each socket-rim record is moved along its own ray from the PLACED cap
        # centre so that its distance from that centre equals its donor distance from the DONOR cap centre. Tangential placement
        # stays the fit's (the rim follows the fitted lids); only the radial component is restored. Records already at their
        # donor radius do not move. The placement rule is the lid-clearance one (cc5_centre = the cap assemble() translates to).
        if p['lid_clearance_placement'] not in LID_CLEARANCE_PLACEMENTS: raise ValueError(f"lid_clearance_placement {p['lid_clearance_placement']} unsupported {LID_CLEARANCE_PLACEMENTS}")
        sets = socket_rim_sets(ex, mk, p['socket_rim_tol_mm']); socket_report = {'policy': 'preserve_donor_radius', 'tol_mm': p['socket_rim_tol_mm'], 'placement': p['lid_clearance_placement'], 'sides': {}, 'unbanded_on_cap': sets['unbanded_on_cap']}
        for role, cidx, side in (('eye_r', 0, 'r'), ('eye_l', 1, 'l')):
            sd_ = sets['sides'][side]; c0 = np.array(sd_['centre']); r0 = sd_['radius_m']; c = CAPC[cidx] if p['lid_clearance_placement'] == 'cc5_centre' else c0; r_shift = r0 * (CAPS[cidx] - 1); r0 = r0 * CAPS[cidx]      # declared cap scale: donor signed distance kept against the scaled cap
            ids = np.array(sd_['ids'], int); before = (np.linalg.norm(fitp[ids] - c, axis=1) - r0) * 1000 if len(ids) else np.zeros(0); pushes = []
            for i in ids:
                rd = float(np.linalg.norm(P[i] - c0)) + r_shift; v = fitp[i] - c; d = float(np.linalg.norm(v))
                if d < 1e-12: continue
                target = c + v / d * rd; pushes.append(float(np.linalg.norm(target - fitp[i]) * 1000)); fitp[i] = target; mode[i] = 'socket_rim'
            after = (np.linalg.norm(fitp[ids] - c, axis=1) - r0) * 1000 if len(ids) else np.zeros(0)
            ring_moves = {}
            if p['socket_radius_rings']:
                # declared route policy: the K rows around the rim keep a graded share of the same donor-radius correction (row k of K: (K-k+1)/(K+1),
                # or 1 when not graded), so the lid margin band keeps the donor's radial profile around the placed eyeball and fades into the fit
                K = int(p['socket_radius_rings']); prev = ids.copy(); seen = set(ids.tolist())
                for k in range(1, K + 1):
                    ring = np.setdiff1d(G.grow(prev, adj, 1), np.array(sorted(seen), int)) if len(prev) else np.zeros(0, int); ring = ring[on_skin[ring] & ~lash[ring] & ~is_frozen[ring]]
                    w = (K - k + 1) / (K + 1) if p['socket_radius_graded'] else 1.0; mv = []
                    for i in ring:
                        rd = float(np.linalg.norm(P[i] - c0)) + r_shift; v = fitp[i] - c; dd = float(np.linalg.norm(v))
                        if dd < 1e-12: continue
                        target = c + v / dd * (dd + w * (rd - dd)); mv.append(float(np.linalg.norm(target - fitp[i]) * 1000)); fitp[i] = target; mode[i] = 'socket_ring'
                    ring_moves[k] = {'records': int(len(ring)), 'weight': w, 'max_move_mm': max(mv, default=0.0)}; seen |= set(ring.tolist()); prev = np.union1d(prev, ring)
            socket_report['sides'][side] = {'ring_moves': ring_moves, 'cap_radius_scale': float(CAPS[cidx]), 'cap_radius_mm': float(r0 * 1000), 'role': role, 'character_side': sd_['character_side'], 'records': int(len(ids)), 'on_cap': len(sd_['on_cap']), 'interior': len(sd_['interior']), 'signed_to_placed_cap_before_mm': {'min': float(before.min()), 'max': float(before.max())} if len(ids) else None,
                                            'signed_to_placed_cap_after_mm': {'min': float(after.min()), 'max': float(after.max())} if len(ids) else None, 'donor_signed_mm': {'min': min(sd_['donor_signed_mm'].values()), 'max': max(sd_['donor_signed_mm'].values())} if len(ids) else None, 'radial_move_mm': {'max': max(pushes, default=0.0), 'median': float(np.median(pushes)) if pushes else 0.0}}
    for ids in dup: fitp[ids] = fitp[ids].mean(0)
    fitp[frozen] = P[frozen]
    q = G.triangle_quality(P, fitp, T); mf = FQ.face_metrics(fitp, T, dup); md = FQ.face_metrics(P, T, dup); nolash = ~lash[T].any(1)
    report = {'fit_metrics_excluding_lash_faces': {k: int((mf[k] & nolash).sum()) for k in ('inverted_local', 'folded_edge', 'sliver')}, 'donor_metrics_excluding_lash_faces': {k: int((md[k] & nolash).sum()) for k in ('inverted_local', 'folded_edge', 'sliver')}, 'lash_faces': int((~nolash).sum()),'params': p, 'registration': reg_report, 'shift_m': shift.tolist(), 'band_top_z': band_top, 'modes': {k: int((mode == k).sum()) for k in ('uv', 'surface', 'uncovered', 'radial', 'blend', 'band', 'frozen', 'lash', 'mouth_rigid', 'lip_relaxed', 'lid_cleared', 'scalp_frozen', 'scalp_blend', 'lip_block', 'lip_block_blend', 'socket_rim', 'socket_ring', 'socket_blend')}, 'radial_hits': radial_hit, 'inverted_by_region': inverted_by_region(mf, md, T, mk), 'laplace_iterations': int(its), 'lid_clearance': lid_report, 'socket_rim': socket_report, 'scalp': scalp_report, 'lip_block': lip_report,
              'max_move_vs_donor_mm': float(np.linalg.norm(fitp - P, axis=1).max() * 1000), 'attachment_max_error_m': float(np.abs(fitp[frozen] - P[frozen]).max()), 'seam_max_gap_m': max((float(np.linalg.norm(fitp[ids] - fitp[ids[0]], axis=1).max()) for ids in dup), default=0.0),
              'triangle_quality_vs_donor': q, 'fit_metrics': {k: int(mf[k].sum()) for k in ('inverted_local', 'folded_edge', 'sliver')}, 'donor_metrics': {k: int(md[k].sum()) for k in ('inverted_local', 'folded_edge', 'sliver')}, 'seconds': round(time.time() - t0, 1)}
    return fitp, mode.astype(str), report, mk


def inverted_by_region(mf, md, T, mk):
    """Where the fit's locally inverted faces sit (regions from the donor's own structure/weights)."""
    regions = {'mouth_ring': np.isin(T, mk['mouth_loop']).any(1), 'eyelid_buckets': (mk['bucket']['upper_eyelid_r'] | mk['bucket']['lower_eyelid_r'] | mk['bucket']['upper_eyelid_l'] | mk['bucket']['lower_eyelid_l'])[T].any(1),
               'lip_buckets': (mk['bucket']['upper_lip'] | mk['bucket']['lower_lip'] | mk['bucket']['mouth_corner_l'] | mk['bucket']['mouth_corner_r'])[T].any(1), 'band_or_frozen': (mk['is_frozen'] | (mk['P'][:, 2] < mk['P'][mk['attachment'], 2].max() + 0.05))[T].any(1),
               'rear_opening_ring': np.isin(T, mk['rear_opening']).any(1), 'lash': mk['lash'][T].any(1)}
    inv_new = mf['inverted_local'] & ~md['inverted_local']; out = {'new_inverted_total': int(inv_new.sum())}
    rest = inv_new.copy()
    for k, m in regions.items(): out[k] = int((inv_new & m).sum()); rest &= ~m
    out['elsewhere'] = int(rest.sum()); out['elsewhere_faces'] = np.flatnonzero(rest).tolist()[:50]; return out


# ---- measurements -----------------------------------------------------------------------------------------------
def _stats(d):
    d = np.asarray(d, float) * 1000; return {'median_mm': float(np.median(d)), 'p95_mm': float(np.percentile(d, 95)), 'max_mm': float(d.max()), 'n': int(len(d))} if len(d) else {'n': 0}


def measure(ex, fitp, mode, mk, src, shift, other_fit=None):
    """Likeness, lips, eyelids, jaw/neck band, texture coverage and rear-opening clearance for one fit; `other_fit`
    (positions of another hypothesis) gives the per-region discrepancy between hypotheses."""
    P, T, U = mk['P'], mk['T'], mk['U']; S0, ST, SU = C.head_arrays(src); S = S0 + shift; soup = G.TriangleSoup(S, ST); n = len(P)
    dist = np.array([soup.nearest(fitp[i])[1] for i in range(n)])
    skin = mk['on_skin'] & ~mk['is_frozen'] & ~mk['lash']; face = mk['face_window'] & skin; band = (mode == 'band')
    SE, r_cc5 = eye_centres_cc5(src); SE = SE + shift
    lids = {}
    for side, cidx in (('r', 0), ('l', 1)):
        m = (mk['bucket'][f'upper_eyelid_{side}'] | mk['bucket'][f'lower_eyelid_{side}']) & mk['on_skin'] & ~mk['lash']      # outer-shell lid records only; lash-strip records (thin cards, one already inside the donor's own cap) reported separately
        sd_after = np.linalg.norm(fitp[m] - SE[cidx], axis=1) - r_cc5; sd_before = np.linalg.norm(P[m] - SE[cidx], axis=1) - r_cc5
        if not m.any(): lids[side] = {'records': 0, 'inside_sphere_after': 0, 'inside_sphere_donor_vs_cc5_sphere': 0, 'note': 'no eyelid-bucket records on the outer shell'}; continue
        lids[side] = {'records': int(m.sum()), 'signed_to_cc5_eye_sphere_after_mm': {'min': float(sd_after.min() * 1000), 'median': float(np.median(sd_after) * 1000)}, 'inside_sphere_after': int((sd_after < 0).sum()), 'inside_sphere_donor_vs_cc5_sphere': int((sd_before < 0).sum()), 'move_mm': _stats(np.linalg.norm(fitp[m] - P[m], axis=1))}
    ml = mk['mouth_loop']; lipsm = mk['bucket']['upper_lip'] | mk['bucket']['lower_lip'] | mk['bucket']['mouth_corner_l'] | mk['bucket']['mouth_corner_r']
    lips = {'mouth_loop_records': int(len(ml)), 'mouth_loop_to_cc5_surface': _stats(dist[ml]), 'mouth_loop_height_donor_mm': float((P[ml, 2].max() - P[ml, 2].min()) * 1000), 'mouth_loop_height_fit_mm': float((fitp[ml, 2].max() - fitp[ml, 2].min()) * 1000),
            'mouth_loop_width_donor_mm': float((P[ml, 0].max() - P[ml, 0].min()) * 1000), 'mouth_loop_width_fit_mm': float((fitp[ml, 0].max() - fitp[ml, 0].min()) * 1000), 'mouth_loop_centroid_shift_mm': ((fitp[ml].mean(0) - P[ml].mean(0)) * 1000).tolist(),
            'lip_bucket_records': int(lipsm.sum()), 'lip_bucket_to_cc5_surface': _stats(dist[lipsm]), 'lip_bucket_move_mm': _stats(np.linalg.norm(fitp[lipsm] - P[lipsm], axis=1))}
    jaw = {'band_records': int(band.sum()), 'band_to_cc5_surface': _stats(dist[band]), 'band_move_mm': _stats(np.linalg.norm(fitp[band] - P[band], axis=1)), 'jaw_bucket_to_cc5_surface': _stats(dist[mk['bucket']['jaw'] & skin]), 'attachment_max_error_m': float(np.abs(fitp[mk['frozen']] - P[mk['frozen']]).max())}
    likeness = {'skin_all': _stats(dist[skin]), 'face_window': _stats(dist[face]), 'outside_face_window': _stats(dist[skin & ~face]), 'donor_before_fit_skin': _stats(np.array([soup.nearest(P[i])[1] for i in np.flatnonzero(skin)]))}
    # texture coverage: Legacy atlas texels (512^2) inside head triangles whose three records are UV/surface-mapped (not band/uncovered/radial)
    R = 512; tri = F.uv_raster(U, T, R); head_tex = tri >= 0; good_face = np.isin(mode, ('uv', 'surface', 'lash'))[T].all(1); good_tex = head_tex & good_face[np.clip(tri, 0, None)]
    any_face = np.isin(mode, ('uv', 'surface', 'lash', 'radial', 'blend', 'mouth_rigid', 'lip_relaxed'))[T].all(1); any_tex = head_tex & any_face[np.clip(tri, 0, None)]
    coverage = {'atlas_size': R, 'head_texels': int(head_tex.sum()), 'texels_with_uv_registered_correspondence': int(good_tex.sum()), 'fraction_uv_registered': float(good_tex.sum() / max(head_tex.sum(), 1)),
                'texels_with_any_source_correspondence': int(any_tex.sum()), 'fraction_any_source_correspondence': float(any_tex.sum() / max(head_tex.sum(), 1)), 'note': 'uv-registered texels can be painted from the CC5 atlas by registration; the rest need the surface bake (mode B) or native fallback', 'records_by_mode': {k: int((mode == k).sum()) for k in np.unique(mode)}}
    # rear opening clearance against the native haircap (untouched)
    hc = LX.submesh(ex, 'haircap'); HP = G.native_to_blender(hc['positions']); HT = np.array(hc['triangles']); hsoup = G.TriangleSoup(HP, HT); ro = mk['rear_opening']
    def clearance(X):
        d = []; outside = 0
        for i in ro:
            q, dd, k = hsoup.nearest_face(X[i]); nrm = np.cross(HP[HT[k, 1]] - HP[HT[k, 0]], HP[HT[k, 2]] - HP[HT[k, 0]]); nrm /= max(np.linalg.norm(nrm), 1e-20)
            cen = HP.mean(0); nrm = nrm if (HP[HT[k]].mean(0) - cen) @ nrm > 0 else -nrm       # outward = away from cap centroid
            d.append(dd); outside += int((X[i] - q) @ nrm > 0)
        return {'to_haircap': _stats(np.array(d)), 'records_outside_cap_surface': outside, 'records': int(len(ro))}
    rear = {'donor': clearance(P), 'fit': clearance(fitp)}
    out = {'likeness': likeness, 'lips': lips, 'eyelids': lids, 'jaw_neck': jaw, 'coverage': coverage, 'rear_opening_vs_haircap': rear, 'eye_radius_mm': {'cc5': r_cc5 * 1000, 'legacy': eye_centres_legacy(ex)[1] * 1000}}
    if other_fit is not None:
        dd = np.linalg.norm(fitp - other_fit, axis=1); out['discrepancy_vs_other'] = {'skin_all': _stats(dd[skin]), 'face_window': _stats(dd[face]), 'mouth_loop': _stats(dd[ml]), 'lips_bucket': _stats(dd[lipsm]), 'eyelids': _stats(dd[mk['bucket']['upper_eyelid_r'] | mk['bucket']['lower_eyelid_r'] | mk['bucket']['upper_eyelid_l'] | mk['bucket']['lower_eyelid_l']]), 'band': _stats(dd[band])}
    return out


# ---- candidate assembly (rigid companions) ---------------------------------------------------------------------
HAIRCAP_LASH_POLICIES = ('untouched', 'follow_lid_rigid', 'follow_lid_record', 'follow_cap_rigid', 'follow_lid_column_rigid', 'follow_lid_column_fit', 'follow_lid_record_bounded')
# follow_lid_column_fit: roots at their fixed (donor-derived) lid correspondences; each column keeps its donor shape and is rotated about its root by the
#   rotation that best fits the column's own per-record lid targets (Kabsch, root pinned) - shape-preserving, target-informed, no face-frame guess
# follow_lid_record_bounded: per-record lid targets, then a bounded regularisation of every column: length clamped to [1-b, 1+b] x donor and the tip
#   direction blended back toward the donor direction rotated by the root frame when it deviates more than the declared angle (LASH_BOUNDS)
LASH_BOUNDS = {'length_ratio': 0.15, 'angle_deg': 20.0}   # follow_lid_column_rigid: piecewise-rigid - each lash column (root record + its tip rows, see lash_card_structure) moves rigidly with the local lid frame at its root (translation = the root's lid displacement, rotation = donor->fitted frame of the nearest lid face): stroke shape kept, root embedding follows the lid   # follow_cap_rigid: each card translated by its eye cap's translation (exact donor card/cap/rim relation when the socket block is donor-shaped)   # the eyelash cards are separate haircap components authored against the donor lid margin; follow_lid_*: each card follows the fitted lid (rigid Kabsch fit of its records' lid-surface targets, or per record)
LASH_CARD_MAX_DIST_M = 0.035
LASH_CARD_EXPECTATIONS = {'profile': 'nhl-legacy-x360-player67', 'haircap_components': 3, 'cards': {'r': {'records': 32, 'faces': 42, 'native_x_sign': -1}, 'l': {'records': 32, 'faces': 42, 'native_x_sign': +1}}, 'rows': 4, 'columns': 8}
# pinned donor topology for the classifier (measured on player_67: hair shell + two 8-column x 4-row lash strips; character right = native x < 0).
# haircap_lash_components(..., expect=LASH_CARD_EXPECTATIONS) refuses any other donor instead of generalising.


def haircap_lash_components(ex, mk, expect=None):
    """Eyelash cards on the haircap: every haircap connected component EXCEPT the largest (the hair shell) whose records
    all lie within LASH_CARD_MAX_DIST_M of exactly one donor eye cap centre and carry an eye palette (eye_control or an
    eyelid bucket) on every record. Refuses (ValueError) a small component that matches neither rule or both caps, so an
    unexpected donor is not silently classified. Returns {'r': ids, 'l': ids, 'components': n, 'shell': ids}."""
    hc = LX.submesh(ex, 'haircap'); HP = G.native_to_blender(hc['positions']); comps = [np.array(c['vertex_ids'], int) for c in LX.connected_components(len(HP), hc['triangles'])]
    pal = LX.palettes_of(hc['joints'], hc['weights']); eye_pal = {s: set(LX.PALETTE_BUCKETS[f'eye_control_{s}']) | set(LX.PALETTE_BUCKETS[f'upper_eyelid_{s}']) | set(LX.PALETTE_BUCKETS[f'lower_eyelid_{s}']) for s in ('r', 'l')}
    centres = {s: sphere_fit(G.native_to_blender(LX.submesh(ex, role)['positions']))[0] for s, role in (('r', 'eye_r'), ('l', 'eye_l'))}
    shell = max(comps, key=len); out = {'r': [], 'l': [], 'components': len(comps), 'shell_records': int(len(shell))}
    for comp in comps:
        if comp is shell: continue
        near = {s: bool((np.linalg.norm(HP[comp] - centres[s], axis=1) < LASH_CARD_MAX_DIST_M).all()) for s in ('r', 'l')}
        pals = {s: all(any(q in eye_pal[s] for q in pal[i]) for i in comp) for s in ('r', 'l')}
        ok = [s for s in ('r', 'l') if near[s] and pals[s]]
        if len(ok) != 1: raise ValueError(f'haircap component of {len(comp)} records is not an unambiguous eyelash card (near {near}, eye palettes {pals})')
        if out[ok[0]]: raise ValueError(f'two haircap components classify as the {ok[0]} eyelash card')
        out[ok[0]] = comp.tolist()
    if expect is not None:
        # fail closed against the pinned topology: component count, per-card record / face counts, character side by native x sign, row / column structure
        HT = np.array(hc['triangles']); HU = np.array(hc['uv']); Xn = np.asarray(hc['positions'], float)
        if out['components'] != expect['haircap_components']: raise ValueError(f"haircap has {out['components']} components, profile {expect['profile']} pins {expect['haircap_components']}")
        for s_, e in expect['cards'].items():
            ids = np.array(out[s_], int); nf = int(np.isin(HT, ids).all(1).sum())
            if len(ids) != e['records'] or nf != e['faces']: raise ValueError(f"{s_} eyelash card has {len(ids)} records / {nf} faces, profile pins {e['records']} / {e['faces']}")
            if len(ids) and np.sign(Xn[ids, 0].mean()) != e['native_x_sign']: raise ValueError(f"{s_} eyelash card lies on the wrong side (native x mean {Xn[ids, 0].mean():.4f})")
            st = lash_card_structure(G.native_to_blender(hc['positions']), HT, HU, ids)
            if len(st['rows_v']) != expect['rows'] or len(st['columns']) != expect['columns']: raise ValueError(f"{s_} eyelash card structure {len(st['rows_v'])} rows x {len(st['columns'])} columns, profile pins {expect['rows']} x {expect['columns']}")
        out['expectations'] = expect
    return out


def lash_card_structure(HP, HT, HU, ids):
    """Rows and columns of one eyelash card strip from its UVs and topology: rows = distinct v values (root row = the
    smallest v, the row that rests on the lid; tips outward), columns = for each root record the chain of edge-neighbours
    with the same u through the successive rows. Refuses (ValueError) a card that is not a clean rows x columns strip."""
    ids = np.array(ids, int); idset = set(ids.tolist()); v = HU[ids, 1]; rows_v = sorted(set(np.round(v, 4).tolist()))
    nb = {int(i): set() for i in ids}
    for f in HT[np.isin(HT, ids).all(1)]:
        for a, b in ((f[0], f[1]), (f[1], f[2]), (f[2], f[0])): nb[int(a)].add(int(b)); nb[int(b)].add(int(a))
    row_of = {int(i): rows_v.index(round(float(HU[i, 1]), 4)) for i in ids}
    roots = [int(i) for i in ids if row_of[int(i)] == 0]; columns = []
    for r in roots:
        chain = [r]
        for k in range(1, len(rows_v)):
            cand = [j for j in nb[chain[-1]] if row_of[j] == k and abs(HU[j, 0] - HU[r, 0]) < 1e-4]
            if len(cand) != 1: raise ValueError(f'eyelash card column from root {r}: {len(cand)} candidates in row {k}')
            chain.append(cand[0])
        columns.append(chain)
    if sorted(sum(columns, [])) != sorted(ids.tolist()): raise ValueError('eyelash card records are not covered exactly once by the columns')
    X = HP[roots]; c = X.mean(0); ax = np.linalg.svd(X - c)[2][0]; order = np.argsort((X - c) @ ax); columns = [columns[i] for i in order]
    return {'rows_v': rows_v, 'columns': columns, 'roots': [c_[0] for c_ in columns], 'tips': [c_[-1] for c_ in columns]}


def _frame(n, e):
    """Orthonormal frame (columns) from a normal and an in-plane direction."""
    n = n / max(np.linalg.norm(n), 1e-12); e = e - (e @ n) * n; e = e / max(np.linalg.norm(e), 1e-12); return np.c_[n, e, np.cross(n, e)]


def lash_card_metrics(ex_donor, ex_cand, mk, cards, placed_caps=None):
    """Per card: root embedding along the lid (signed distance of every root record to the candidate's outer head skin,
    donor 0 = resting on it; negative = under the skin), card stretch (root-row spacing and column length ratios vs the
    donor), column orientation (angle of each root->tip column to the local skin normal, donor vs candidate), cap
    clearance (min signed distance of every card record to the placed cap sphere), card/head crossings split by row
    band. Pure geometry (no alpha); the caller adds alpha-aware visibility. `placed_caps` = {side: (centre, radius)}."""
    hd = LX.submesh(ex_donor, 'haircap'); HP0 = G.native_to_blender(hd['positions']); HT = np.array(hd['triangles']); HU = np.array(hd['uv']); HP1 = G.native_to_blender(LX.submesh(ex_cand, 'haircap')['positions'])
    P0 = mk['P']; T = mk['T']; P1 = G.native_to_blender(LX.submesh(ex_cand, 'head')['positions']); outer = mk['outer_faces']; out = {}
    from . import legacy_qualify as LQ
    for s, role in (('r', 'eye_r'), ('l', 'eye_l')):
        ids = np.array(cards[s], int)
        if not len(ids): out[s] = {'records': 0}; continue
        st = lash_card_structure(HP0, HT, HU, ids); roots = np.array(st['roots']); tips = np.array(st['tips'])
        def emb(P, HP):
            soup = G.TriangleSoup(P, T[outer]); fn = FQ.face_normals(P, T[outer]); vals = []
            for r in roots:
                q, d, f = soup.nearest_face(HP[r]); vals.append(float((HP[r] - q) @ fn[f] * 1000))
            return vals
        def orient(P, HP):
            soup = G.TriangleSoup(P, T[outer]); fn = FQ.face_normals(P, T[outer]); angs = []
            for r, t in zip(roots, tips):
                q, d, f = soup.nearest_face(HP[r]); v = HP[t] - HP[r]; v = v / max(np.linalg.norm(v), 1e-12); angs.append(float(np.degrees(np.arccos(np.clip(v @ fn[f], -1, 1)))))
            return angs
        e0, e1 = emb(P0, HP0), emb(P1, HP1); o0, o1 = orient(P0, HP0), orient(P1, HP1)
        sp0 = np.linalg.norm(HP0[roots[1:]] - HP0[roots[:-1]], axis=1); sp1 = np.linalg.norm(HP1[roots[1:]] - HP1[roots[:-1]], axis=1); cl0 = np.linalg.norm(HP0[tips] - HP0[roots], axis=1); cl1 = np.linalg.norm(HP1[tips] - HP1[roots], axis=1)
        cf = np.flatnonzero(np.isin(HT, ids).all(1)); rows = {int(i): st['rows_v'].index(round(float(HU[i, 1]), 4)) for i in ids}; root_faces = cf[np.array([min(rows[int(v)] for v in HT[f]) == 0 for f in cf])]; tip_faces = np.setdiff1d(cf, root_faces)
        xr = LQ.cross_intersections(HP1, HT, P1, T, root_faces); xt = LQ.cross_intersections(HP1, HT, P1, T, tip_faces)
        cap = {}
        if placed_caps:
            c, r = placed_caps[s]; sd = (np.linalg.norm(HP1[ids] - c, axis=1) - r) * 1000; sd0 = (np.linalg.norm(HP0[ids] - c, axis=1) - r) * 1000; cap = {'min_signed_mm': float(sd.min()), 'records_inside': int((sd < 0).sum()), 'roots_min_signed_mm': float(sd[np.isin(ids, roots)].min())}
        out[s] = {'records': int(len(ids)), 'columns': len(st['columns']), 'rows': len(st['rows_v']), 'root_embedding_mm': {'donor': e0, 'candidate': e1, 'candidate_max_abs': float(np.abs(e1).max()), 'candidate_mean': float(np.mean(e1))},
                  'root_spacing_ratio_vs_donor': {'min': float((sp1 / sp0).min()), 'max': float((sp1 / sp0).max())}, 'column_length_ratio_vs_donor': {'min': float((cl1 / cl0).min()), 'max': float((cl1 / cl0).max())},
                  'column_angle_to_skin_normal_deg': {'donor': o0, 'candidate': o1, 'max_abs_change': float(np.abs(np.array(o1) - np.array(o0)).max())}, 'move_mm': {'median': float(np.median(np.linalg.norm(HP1[ids] - HP0[ids], axis=1)) * 1000), 'max': float(np.linalg.norm(HP1[ids] - HP0[ids], axis=1).max() * 1000)},
                  'card_head_crossings': {'root_band_faces': len(xr), 'tip_faces': len(xt), 'note': 'root-band crossings = the card entering the lid (embedding); tip crossings = strokes cut by skin'}, 'cap_clearance': cap}
    return out


def kabsch(A, B):
    """Rigid (R, t) minimising |R A + t - B|^2 (no scale)."""
    A = np.asarray(A, float); B = np.asarray(B, float); ca = A.mean(0); cb = B.mean(0); H = (A - ca).T @ (B - cb); U, S, Vt = np.linalg.svd(H); d = np.sign(np.linalg.det(Vt.T @ U.T))
    R = Vt.T @ np.diag([1, 1, d]) @ U.T; return R, cb - R @ ca


def move_lash_cards(ex, fitp, mk, policy, cards=None, cap_translations=None):
    """Haircap positions with the eyelash cards following the fitted lid: each card record's target = its donor position +
    the displacement of the nearest donor lid-surface point (outer-shell head faces within 30 mm of that eye's cap
    centre, barycentric, donor -> fitted). 'follow_lid_record' applies the targets; 'follow_lid_rigid' fits one rigid
    transform per card to them (shape preserved) and reports the residual. Returns (haircap positions Blender, report)."""
    if policy not in HAIRCAP_LASH_POLICIES: raise ValueError(f'haircap_lash_policy {policy} unsupported {HAIRCAP_LASH_POLICIES}')
    hc = LX.submesh(ex, 'haircap'); HP = G.native_to_blender(hc['positions']); out = HP.copy(); rep = {'policy': policy, 'sides': {}}
    if policy == 'untouched': return out, rep
    cards = cards or haircap_lash_components(ex, mk); P, T = mk['P'], mk['T']; lash = mk['lash']
    for s, role in (('r', 'eye_r'), ('l', 'eye_l')):
        ids = np.array(cards[s], int)
        if not len(ids): rep['sides'][s] = {'records': 0}; continue
        c0 = sphere_fit(G.native_to_blender(LX.submesh(ex, role)['positions']))[0]
        if policy == 'follow_cap_rigid':
            t = np.asarray(cap_translations[role], float); out[ids] = HP[ids] + t
            rep['sides'][s] = {'records': int(len(ids)), 'translation_mm': (t * 1000).tolist(), 'move_mm': {'median': float(np.linalg.norm(t) * 1000), 'max': float(np.linalg.norm(t) * 1000)}, 'rigid_residual_mm': {'median': 0.0, 'max': 0.0}}; continue
        faces = np.flatnonzero(mk['outer_faces'] & ~lash[T].all(1) & (np.linalg.norm(P[T].mean(1) - c0, axis=1) < 0.03)); soup = G.TriangleSoup(P, T[faces]); tgt = np.zeros((len(ids), 3)); dist = np.zeros(len(ids))
        for k, i in enumerate(ids):
            q, d, f = soup.nearest_face(HP[i]); t = T[faces[f]]; b = _bary(q, *P[t]); tgt[k] = HP[i] + (fitp[t] * b[:, None]).sum(0) - q; dist[k] = d
        if policy == 'follow_lid_record': out[ids] = tgt; resid = np.zeros(len(ids))
        elif policy == 'follow_lid_column_fit':
            hc_ = LX.submesh(ex, 'haircap'); st = lash_card_structure(HP, np.array(hc_['triangles']), np.array(hc_['uv']), ids)
            for col in st['columns']:
                r = col[0]; k = [loc_ for loc_, i in enumerate(ids) if i == r][0]; root_t = tgt[k]; rel = HP[col] - HP[r]; ct = np.array([tgt[[loc_ for loc_, i in enumerate(ids) if i == j][0]] for j in col]) - root_t
                H = rel[1:].T @ ct[1:]; U_, S_, Vt_ = np.linalg.svd(H); dd = np.sign(np.linalg.det(Vt_.T @ U_.T)); Rm = Vt_.T @ np.diag([1, 1, dd]) @ U_.T     # rotation about the pinned root (no translation, no scale)
                for j in col: out[j] = root_t + Rm @ (HP[j] - HP[r])
            resid = np.linalg.norm(out[ids] - tgt, axis=1); rep['sides'][s] = {'columns': len(st['columns']), 'policy_note': 'column rigid about its root; rotation = Kabsch fit of the column tips to their per-record lid targets'}
        elif policy == 'follow_lid_record_bounded':
            hc_ = LX.submesh(ex, 'haircap'); st = lash_card_structure(HP, np.array(hc_['triangles']), np.array(hc_['uv']), ids); fn_d = FQ.face_normals(P, T[faces]); fn_f = FQ.face_normals(fitp, T[faces]); out[ids] = tgt; clamped = 0
            for col in st['columns']:
                r = col[0]; q, d, f = soup.nearest_face(HP[r]); Fd = _frame(fn_d[f], P[T[faces[f]][1]] - P[T[faces[f]][0]]); Ff = _frame(fn_f[f], fitp[T[faces[f]][1]] - fitp[T[faces[f]][0]]); Rm = Ff @ Fd.T
                for j in col[1:]:
                    v_ref = Rm @ (HP[j] - HP[r]); v = out[j] - out[r]; l0 = np.linalg.norm(v_ref); l = max(np.linalg.norm(v), 1e-12); u = v / l; u0 = v_ref / max(l0, 1e-12)
                    ang = np.degrees(np.arccos(np.clip(u @ u0, -1, 1))); moved = False
                    if ang > LASH_BOUNDS['angle_deg']:
                        # rotate u toward u0 so the residual angle equals the bound (slerp in the plane of u, u0)
                        t_ = (ang - LASH_BOUNDS['angle_deg']) / ang; w_ = u0 - (u0 @ u) * u; w_ = w_ / max(np.linalg.norm(w_), 1e-12); phi = np.radians(ang) * t_; u = np.cos(phi) * u + np.sin(phi) * w_; moved = True
                    l = float(np.clip(l, (1 - LASH_BOUNDS['length_ratio']) * l0, (1 + LASH_BOUNDS['length_ratio']) * l0)); out[j] = out[r] + u * l; clamped += int(moved or abs(l - np.linalg.norm(v)) > 1e-12)
            resid = np.linalg.norm(out[ids] - tgt, axis=1); rep['sides'][s] = {'columns': len(st['columns']), 'records_clamped': clamped, 'bounds': LASH_BOUNDS, 'policy_note': 'per-record lid targets; tip length and angle to the root-frame donor direction clamped to the declared bounds'}
        elif policy == 'follow_lid_column_rigid':
            hc_ = LX.submesh(ex, 'haircap'); st = lash_card_structure(HP, np.array(hc_['triangles']), np.array(hc_['uv']), ids); fn_d = FQ.face_normals(P, T[faces]); fn_f = FQ.face_normals(fitp, T[faces]); loc = {int(i): k for k, i in enumerate(ids)}
            for col in st['columns']:
                r = col[0]; q, d, f = soup.nearest_face(HP[r]); t = T[faces[f]]; b = _bary(q, *P[t]); root_t = HP[r] + (fitp[t] * b[:, None]).sum(0) - q
                # local lid frame at the root: face normal + the edge direction of the nearest face, donor -> fitted (rotation only)
                e_d = P[t[1]] - P[t[0]]; e_f = fitp[t[1]] - fitp[t[0]]; Fd = _frame(fn_d[f], e_d); Ff = _frame(fn_f[f], e_f); Rm = Ff @ Fd.T
                for j in col: out[j] = root_t + Rm @ (HP[j] - HP[r])
            resid = np.linalg.norm(out[ids] - tgt, axis=1); rep['sides'][s] = {'columns': len(st['columns']), 'policy_note': 'each column rigid in its root lid frame; residual = deviation from the per-record lid targets'}
        else:
            R, t = kabsch(HP[ids], tgt); out[ids] = HP[ids] @ R.T + t; resid = np.linalg.norm(out[ids] - tgt, axis=1)
            rep['sides'][s] = {'rotation_deg': float(np.degrees(np.arccos(np.clip((np.trace(R) - 1) / 2, -1, 1)))), 'translation_mm': (t * 1000).tolist() if False else float(np.linalg.norm(out[ids].mean(0) - HP[ids].mean(0)) * 1000)}
        rep['sides'].setdefault(s, {}).update({'records': int(len(ids)), 'donor_distance_to_lid_surface_mm': {'median': float(np.median(dist) * 1000), 'max': float(dist.max() * 1000)}, 'move_mm': {'median': float(np.median(np.linalg.norm(out[ids] - HP[ids], axis=1)) * 1000), 'max': float(np.linalg.norm(out[ids] - HP[ids], axis=1).max() * 1000)},
                                              'rigid_residual_mm': {'median': float(np.median(resid) * 1000), 'max': float(resid.max() * 1000)}, 'mean_displacement_mm': ((out[ids] - HP[ids]).mean(0) * 1000).tolist()})
    return out, rep


def assemble(ex, fitp, src, shift, mk, scale_eyes=False, eye_placement='translate_to_cc5_centre', haircap_lash_policy='untouched', cap_centre_override=None, cards=None, cap_radius_scale=None):
    """Candidate exchange: fitted head, eye caps translated onto the CC5 eye centres (scale only when asked; the
    ratio is reported either way) or left untouched (`eye_placement='untouched'`: the residual CC5-vs-cap centre
    offset is reported instead), mouthbag translated by the mouth-loop centroid delta (Studio baseline policy),
    haircap untouched. Returns (candidate, moves)."""
    if eye_placement not in EYE_PLACEMENTS: raise ValueError(f'eye_placement {eye_placement} unsupported {EYE_PLACEMENTS}')
    if scale_eyes and cap_radius_scale is not None: raise ValueError('scale_eyes (CC5-radius match) and cap_radius_scale (declared scale) are exclusive')
    CAPS = cap_radius_scales({'cap_radius_scale': cap_radius_scale})
    cand = LX.with_positions(ex, 'head', fitp); SE, r_cc5 = eye_centres_cc5(src); SE = placed_cap_centres(SE, shift, cap_centre_override); moves = {}
    for role, cidx in (('eye_r', 0), ('eye_l', 1)):
        Pe = G.native_to_blender(LX.submesh(ex, role)['positions']); c, r, _ = sphere_fit(Pe); t = SE[cidx] - c
        if eye_placement == 'untouched':
            moves[role] = {'translation_m': [0.0, 0.0, 0.0], 'translation_mm': 0.0, 'scale_applied': 1.0, 'radius_ratio_cc5_over_legacy': float(r_cc5 / r), 'cc5_centre_offset_m': t.tolist(), 'cc5_centre_offset_mm': float(np.linalg.norm(t) * 1000), 'policy': 'untouched native cap (head-rigid, palette 8); CC5 centre offset reported only'}; continue
        s_ = float(r_cc5 / r) if scale_eyes else float(CAPS[cidx]); Pn = (Pe - c) * s_ + SE[cidx]          # isotropic scale about the cap's OWN fitted centre, then the translation (topology / UV / indices / weights untouched); s_ = 1 -> exactly Pe + t
        cand = LX.with_positions(cand, role, Pn); moves[role] = {'translation_m': t.tolist(), 'translation_mm': float(np.linalg.norm(t) * 1000), 'scale_applied': s_, 'scale_centre_m': c.tolist(), 'radius_ratio_cc5_over_legacy': float(r_cc5 / r), 'radius_after_mm': float(r * s_ * 1000), 'policy': 'translate to CC5 sphere centre' + (' + uniform scale to the CC5 radius' if scale_eyes else (' + DECLARED isotropic scale about the cap centre' if s_ != 1.0 else ' (no scale: baseline policy)'))}
    ml = mk['mouth_loop']; t = fitp[ml].mean(0) - mk['P'][ml].mean(0); Pm = G.native_to_blender(LX.submesh(ex, 'mouthbag')['positions']) + t
    cand = LX.with_positions(cand, 'mouthbag', Pm); moves['mouthbag'] = {'translation_m': t.tolist(), 'translation_mm': float(np.linalg.norm(t) * 1000), 'policy': 'translate by the head mouth-loop centroid delta (Studio baseline policy); no scale, no reshape'}
    if haircap_lash_policy == 'untouched': moves['haircap'] = {'policy': 'untouched native geometry'}
    else:
        HPn, hrep = move_lash_cards(ex, fitp, mk, haircap_lash_policy, cards=cards, cap_translations={r: np.array(moves[r]['translation_m']) for r in ('eye_r', 'eye_l')}); cand = LX.with_positions(cand, 'haircap', HPn)
        moves['haircap'] = {'policy': f'eyelash cards {haircap_lash_policy}; hair shell untouched', **hrep}
    return cand, moves
