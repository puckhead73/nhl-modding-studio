"""Coupled shared-edge crease (signed dihedral) term for the Legacy constrained eye refit (numpy only).

Why: face-wise orientation / area hinges are independent per face, so two neighbours can each stay within their own
hinge while counter-rotating into a knife edge (measured: upper-lid crease pairs sharpened from 41-87 deg to 91-159 deg,
docs/LEGACY_EYE_ALIGNMENT_REVIEW_2026-09-15.md). The dihedral couples the two faces across their shared edge.

Signed dihedral. For an interior edge (a, b) shared by faces f = (a, b, c) (edge in f's winding order) and g (third vertex
d), with unit face normals n_f, n_g (from each face's own winding) and unit edge direction e = (b - a)/|b - a|:
    theta = atan2((n_f x n_g) . e, n_f . n_g)      in (-pi, pi]
theta = 0 is flat; the sign distinguishes a ridge from a valley for THIS edge orientation, so a reference concavity cannot
be traded for an equally sharp convexity (a fold-over) without leaving its envelope. The same orientation is used for the
reference states, so only consistency matters, not a global convention.

Envelope per edge from two reference states (declared by the caller; the eye refit uses the DONOR and the FIT INPUT):
    lo = min(theta_ref) - margin,  hi = max(theta_ref) + margin
    residual r = max(0, theta - hi) + min(0, theta - lo);  E = sum r^2
Natural donor creases keep their own reference angle (no blanket limit); newly sharpened edges beyond the margin are
penalised; an edge classification (inherited crease / fan / interior / smooth) is reported for reading, not used as a waiver.

Edge set: every interior edge of a declared face set PLUS the edges of the faces adjacent to it (the surround), so a fold
cannot be pushed just outside the checked support (edges whose two faces are both fully fixed cannot change and cost
nothing). Degenerate faces / edges are clamped (|N|, |e| >= eps) so the energy and gradient stay finite; the +-pi branch cut is
moved to the antipode of each edge's envelope midpoint (wrap_to_envelope), so reference creases near 180 deg are continuous.
"""
import numpy as np


def edge_structure(T, face_ids, all_faces_mask=None):
    """Interior edges among `face_ids` and their surround (faces sharing an edge with them), restricted to faces where
    `all_faces_mask` is true (None = all). Returns dict with arrays a, b, c, d (record ids), f, g (face ids), where c is
    f's third vertex and (a, b) follows f's winding; d is g's third vertex."""
    T = np.asarray(T); face_ids = np.asarray(face_ids, int); allowed = np.ones(len(T), bool) if all_faces_mask is None else np.asarray(all_faces_mask, bool)
    ef = {}
    for fi in np.flatnonzero(allowed):
        t = T[fi]
        for k in range(3):
            key = (int(min(t[k], t[(k + 1) % 3])), int(max(t[k], t[(k + 1) % 3]))); ef.setdefault(key, []).append(fi)
    core = set(face_ids.tolist()); rows = []
    for key, fs in ef.items():
        if len(fs) != 2: continue
        f, g = fs
        if f not in core and g not in core: continue          # neither face in the support: this edge cannot change (kept out: it would only add zero rows)
        if g in core and f not in core: f, g = g, f
        tf = T[f]; k = [i for i in range(3) if (int(tf[i]), int(tf[(i + 1) % 3])) in (key, key[::-1])][0]
        a, b, c = int(tf[k]), int(tf[(k + 1) % 3]), int(tf[(k + 2) % 3]); tg = T[g]; d = int([v for v in tg if v not in (a, b)][0])
        rows.append((a, b, c, d, int(f), int(g)))
    R = np.array(rows, int).reshape(-1, 6)
    return {'a': R[:, 0], 'b': R[:, 1], 'c': R[:, 2], 'd': R[:, 3], 'f': R[:, 4], 'g': R[:, 5], 'n': len(R)}


def _unit_normals(T, X, fids, eps):
    t = T[fids]; N = np.cross(X[t[:, 1]] - X[t[:, 0]], X[t[:, 2]] - X[t[:, 0]]); nl = np.maximum(np.linalg.norm(N, axis=1), eps); return N / nl[:, None], nl, t


def signed_dihedrals(X, T, ES, eps=1e-12):
    """Signed dihedral per edge (radians) on positions X (any units)."""
    if ES['n'] == 0: return np.zeros(0)
    nf, _, _ = _unit_normals(T, X, ES['f'], eps); ng, _, _ = _unit_normals(T, X, ES['g'], eps); e = X[ES['b']] - X[ES['a']]; e = e / np.maximum(np.linalg.norm(e, axis=1), eps)[:, None]
    return np.arctan2((np.cross(nf, ng) * e).sum(1), (nf * ng).sum(1))


def wrap_to_envelope(theta, lo, hi):
    """Unwrap theta around the envelope midpoint: the branch cut of atan2 (+-pi) is moved to the antipode of the
    midpoint, so a crease whose reference sits near +-180 deg does not jump by 2 pi across the cut. The energy is
    then continuous everywhere except at the antipode, which lies outside any envelope narrower than 360 deg."""
    mid = 0.5 * (lo + hi); return mid + np.arctan2(np.sin(theta - mid), np.cos(theta - mid))


def envelope(theta_refs, margin_rad):
    """Per-edge [lo, hi] from a list of reference dihedral arrays (each unwrapped to the first reference's branch)."""
    R = np.vstack(theta_refs); R = R[0] + np.arctan2(np.sin(R - R[0]), np.cos(R - R[0])); return R.min(0) - margin_rad, R.max(0) + margin_rad


def envelope_donor_signed(theta_donor, margin_rad, sign_rad, free_rad=None):
    """Single anatomical reference policy (2026-09-16 cap-radius study, solver AND judge). Per edge:
      meaningful crease edge (|donor| >= sign_rad): [donor - margin, donor + margin] clipped to the donor's SIGN (a valley
                stays a valley: lo >= 0 for a positive donor crease, hi <= 0 for a negative one) - identity and sign from the donor
      smooth edge (|donor| < sign_rad): explicit shape adaptation - any angle up to the crease threshold is allowed:
                [min(donor - margin, -free), max(donor + margin, free)] (free = free_rad, default sign_rad), so the fit may reshape
                the region freely but may not create a NEW crease sharper than max(|donor| + margin, free)
    No fit-input union: a damaged fit input never legitimises its own opened fold. Returns (lo, hi, is_crease_edge).
    Measured basis (player_67 x Turgeon raw A0 fit, cap_characterise / pilot 01): the correspondence itself moves p50 14-15 deg and
    p95 66-69 deg from the donor angles over the eye-region edges, so a donor +- margin envelope on EVERY edge (the pilot's
    sign 15 deg policy: 118 / 138 edges outside per side) constrains the intended reshaping, not folds; the 60 deg class is the
    established `inherited_crease` threshold (classify_edges)."""
    th = np.asarray(theta_donor, float); free = sign_rad if free_rad is None else free_rad; lo = th - margin_rad; hi = th + margin_rad; crease = np.abs(th) >= sign_rad
    lo = np.where(crease & (th > 0), np.maximum(lo, 0.0), lo); hi = np.where(crease & (th < 0), np.minimum(hi, 0.0), hi)
    lo = np.where(crease, lo, np.minimum(lo, -free)); hi = np.where(crease, hi, np.maximum(hi, free))
    return lo, hi, crease


def sign_reversals(theta, theta_donor, is_crease_edge):
    """Mask of crease edges (per is_crease_edge) whose signed dihedral on `theta` has the opposite sign to the donor's
    (or is flat within 1e-9 while the donor is a crease). Independent of the envelope: a separate anatomical check."""
    th = np.asarray(theta, float); td = np.asarray(theta_donor, float)
    return np.asarray(is_crease_edge, bool) & (np.sign(th) != np.sign(td))


def crease_energy(X, T, ES, lo, hi, eps=1e-12):
    """E = sum r^2 with r = max(0, theta - hi) + min(0, theta - lo) and its gradient (n, 3) w.r.t. every record of X
    (records not touching a penalised edge get 0). Analytic: d theta = cos * d(sin) - sin * d(cos) with
    sin = (n_f x n_g) . e, cos = n_f . n_g; d(n . u)/d(vertex p of (p, q, r)) = (q - r) x (u - (n . u) n) / |N|;
    d(w . e)/d(b) = (w - (w . e) e)/|b - a| = -d/d(a)."""
    n = len(X); g = np.zeros((n, 3))
    if ES['n'] == 0: return 0.0, g, np.zeros(0)
    nf, lf, tf = _unit_normals(T, X, ES['f'], eps); ng, lg, tg = _unit_normals(T, X, ES['g'], eps)
    ev = X[ES['b']] - X[ES['a']]; el = np.maximum(np.linalg.norm(ev, axis=1), eps); e = ev / el[:, None]
    w = np.cross(nf, ng); s = (w * e).sum(1); c = (nf * ng).sum(1); theta = np.arctan2(s, c); theta = wrap_to_envelope(theta, lo, hi)
    r = np.maximum(0, theta - hi) + np.minimum(0, theta - lo); E = float((r ** 2).sum()); act = r != 0
    if not act.any(): return E, g, theta
    dE = 2 * r                                                             # dE/dtheta
    # dtheta = c * ds - s * dc
    u_f_sin = np.cross(ng, e); u_g_sin = np.cross(e, nf)                   # sin = n_f . (n_g x e) = n_g . (e x n_f)
    def face_grads(nh, nl, t, u, coef):
        """gradient contributions of coef * (nh . u) to the three vertices of faces t."""
        gu = (u - (nh * u).sum(1)[:, None] * nh) / nl[:, None]; p, q, rr = X[t[:, 0]], X[t[:, 1]], X[t[:, 2]]
        for slot, gv in ((0, np.cross(q - rr, gu)), (1, np.cross(rr - p, gu)), (2, np.cross(p - q, gu))): np.add.at(g, t[act, slot], (coef[:, None] * gv)[act])
    face_grads(nf, lf, tf, u_f_sin, dE * c); face_grads(ng, lg, tg, u_g_sin, dE * c)          # c * d(sin) through n_f and n_g
    face_grads(nf, lf, tf, ng, -dE * s); face_grads(ng, lg, tg, nf, -dE * s)                   # -s * d(cos) through n_f and n_g
    de = (w - s[:, None] * e) / el[:, None]; ge = (dE * c)[:, None] * de                       # c * d(sin) through the edge direction
    np.add.at(g, ES['b'][act], ge[act]); np.add.at(g, ES['a'][act], -ge[act])
    return E, g, theta


def classify_edges(ES, theta_donor, lash_face_mask, outer_face_mask, crease_deg=60.0):
    """Reading aid per edge: 'fan' (a lash-fan face on either side), 'interior' (a non-outer face on either side),
    'inherited_crease' (|donor dihedral| >= crease_deg), else 'smooth'."""
    out = np.full(ES['n'], 'smooth', object); f, g = ES['f'], ES['g']
    out[np.abs(theta_donor) >= np.radians(crease_deg)] = 'inherited_crease'
    out[~outer_face_mask[f] | ~outer_face_mask[g]] = 'interior'
    out[lash_face_mask[f] | lash_face_mask[g]] = 'fan'
    return out
