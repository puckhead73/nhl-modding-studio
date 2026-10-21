"""Sphere-based eye parameterisation and donor-cap placement (port of experiments/claude/eyes)."""
import numpy as np


def sphere_fit(P):
    """Algebraic least-squares sphere: returns (centre, radius, residuals)."""
    P = np.asarray(P, float); A = np.c_[2 * P, np.ones(len(P))]; b = (P ** 2).sum(1)
    x, *_ = np.linalg.lstsq(A, b, rcond=None); c = x[:3]; r = float(np.sqrt(x[3] + (c ** 2).sum()))
    return c, r, np.linalg.norm(P - c, axis=1) - r


def place_cap(cap_positions, target_centre, target_radius):
    """Translate a donor eye cap's sphere centre onto the target centre and scale uniformly by the radius ratio."""
    P = np.asarray(cap_positions, float); c, r, _ = sphere_fit(P)
    return (P - c) * (target_radius / r) + np.asarray(target_centre, float), {'donor_centre': c.tolist(), 'donor_radius': r, 'scale': target_radius / r}


def lid_penetration(cap_positions, centre, radius, lid_positions, forward=(0, -1, 0)):
    """Lid records inside the cap's angular extent and inside its sphere; depth in metres. Relative metric:
    compare against the untouched donor's own value rather than reading it as an absolute."""
    Q = np.asarray(cap_positions, float); c = np.asarray(centre, float); fwd = np.asarray(forward, float)
    u = (Q - c) / np.linalg.norm(Q - c, axis=1)[:, None]; cap = np.degrees(np.arccos(np.clip(u @ fwd, -1, 1))).max()
    v = np.asarray(lid_positions, float) - c; ang = np.degrees(np.arccos(np.clip((v / np.linalg.norm(v, axis=1)[:, None]) @ fwd, -1, 1))); rad = np.linalg.norm(v, axis=1)
    inside = (ang < cap) & (rad < radius); depth = (radius - rad)[inside]
    return {'lid_records': int(len(v)), 'penetrating': int(inside.sum()), 'max_depth': float(depth.max()) if inside.any() else 0.0, 'cap_half_angle_deg': float(cap)}
