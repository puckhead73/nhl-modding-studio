"""Fit-independent defect classification across donor / fit / after states (numpy only).

Port of experiments/claude/measure/flip_classify.py. For every triangle: area ratio, min angle, fold score, max
dihedral, exact self-intersection partners (COPLANAR pairs are not detected - coverage unknown), per state. A face is
flagged if it flips vs the fit or the donor, or trips any local metric in any state, or intersects. Each flagged face
gets a region label and a provenance class: inherited_donor / inherited_fit / introduced / resolved / sign_only.
The 'skin' / 'inner' / 'oral' shell label comes from the untouched donor's outer-shell test.
"""
import numpy as np
from . import face_quality as FQ, geometry as G

REGION_HEIGHTS = {'neck_lower_below': 1.70, 'jaw_below': 1.775, 'chin_below': 1.80, 'crown_above': 1.90}   # measured on the NHL atlas-A donors (Blender z, metres)


def classify(donor, fit, after, T, uv, oral_ids, lash_mask, ear_core_mask, mode_labels=None, heights=REGION_HEIGHTS):
    """donor/fit/after: head positions (Blender frame) with identical topology T. Returns {'summary', 'records', 'lists'}."""
    n = len(donor); dup = G.seam_groups(donor)
    oral = np.zeros(n, bool); oral[np.asarray(list(oral_ids), int)] = True
    frozen = G.grow(G.bottom_loop(donor, T), G.adjacency(n, T, dup), 2); is_frozen = np.zeros(n, bool); is_frozen[frozen] = True
    lash = np.asarray(lash_mask, bool); ear_core = np.asarray(ear_core_mask, bool)

    def region_of_face(fi):
        v = T[fi]; c = donor[v].mean(0); z = c[2]; side = 'L' if c[0] > 0.01 else ('R' if c[0] < -0.01 else 'C')
        if oral[v].any(): return 'oral'
        if lash[v].any(): return 'lash_strip_' + side
        if ear_core[v].any(): return 'ear_' + side
        if is_frozen[v].all(): return 'frozen_collar'
        if z < heights['neck_lower_below']: return 'neck_lower_' + side
        if z < heights['jaw_below']: return ('jaw_underside_' + side) if c[1] < 0.02 else ('neck_rear_' + side)
        if z < heights['chin_below']: return ('chin_' + side) if c[1] < -0.04 else ('jaw_angle_' + side if c[1] < 0.03 else 'nape_' + side)
        if c[1] > 0.04: return 'scalp_rear_' + side
        if z > heights['crown_above']: return 'crown_' + side
        return ('temple_' + side) if abs(c[0]) > 0.055 else ('face_' + side)

    outer = FQ.outer_shell_faces(donor, T); oralf = oral[T].any(1); fcls = np.where(oralf, 'oral', np.where(outer, 'skin', 'inner'))
    states = {'donor': donor, 'fit': fit, 'after': after}
    metrics = {k: FQ.face_metrics(P, T, dup) for k, P in states.items()}
    areas = {k: m['area'] for k, m in metrics.items()}
    collapsed = {k: (areas['donor'] > 1e-12) & (areas[k] < 0.05 * areas['donor']) for k in ('fit', 'after')}
    fn = {k: FQ.face_normals(P, T) for k, P in states.items()}
    flip_after_fit = (fn['after'] * fn['fit']).sum(1) < 0; flip_after_donor = (fn['after'] * fn['donor']).sum(1) < 0; flip_fit_donor = (fn['fit'] * fn['donor']).sum(1) < 0

    def local_defect(k, fi):
        m = metrics[k]; d = []
        if m['inverted_local'][fi]: d.append('inverted_local')
        if m['folded_edge'][fi]: d.append('folded_edge')
        if m['sliver'][fi]: d.append('sliver')
        if k != 'donor' and collapsed[k][fi]: d.append('collapsed')
        return d

    inter = {k: FQ.self_intersections(P, T, merge_groups=dup) for k, P in states.items()}
    inter_faces = {k: {} for k in states}
    for k, pairs in inter.items():
        for i, j in pairs: inter_faces[k].setdefault(i, []).append(j); inter_faces[k].setdefault(j, []).append(i)
    flag_any = flip_after_fit | flip_after_donor | flip_fit_donor
    for k in states:
        m = metrics[k]; flag_any |= m['inverted_local'] | m['folded_edge'] | m['sliver']
        flag_any[list(inter_faces[k].keys())] = True
    for k in ('fit', 'after'): flag_any |= collapsed[k]
    records = []
    for fi in np.flatnonzero(flag_any):
        fi = int(fi); d = {k: local_defect(k, fi) + (['self_intersect'] if fi in inter_faces[k] else []) for k in states}
        if d['after'] and not d['fit'] and not d['donor']: cls = 'introduced'
        elif d['after'] and d['donor']: cls = 'inherited_donor'
        elif d['after'] and d['fit']: cls = 'inherited_fit'
        elif (d['fit'] or d['donor']) and not d['after']: cls = 'resolved'
        else: cls = 'sign_only'
        rec = {'face': fi, 'vertices': T[fi].tolist(), 'region': region_of_face(fi), 'shell': str(fcls[fi]), 'class': cls,
               'flip_after_vs_fit': bool(flip_after_fit[fi]), 'flip_after_vs_donor': bool(flip_after_donor[fi]), 'flip_fit_vs_donor': bool(flip_fit_donor[fi]),
               'defects': d, 'centre_after_m': after[T[fi]].mean(0).round(4).tolist(),
               'metrics': {k: {'area_mm2': round(float(areas[k][fi] * 1e6), 4), 'min_angle_deg': round(float(metrics[k]['min_angle_deg'][fi]), 2), 'fold_dot': round(float(metrics[k]['fold_dot'][fi]), 3), 'max_dihedral_deg': round(float(metrics[k]['max_dihedral_deg'][fi]), 1), 'intersects': inter_faces[k].get(fi, [])} for k in states}}
        if mode_labels is not None: rec['neck_modes'] = [str(x) for x in np.asarray(mode_labels)[T[fi]]]
        records.append(rec)

    def count(pred): return int(sum(1 for r in records if pred(r)))

    summary = {'faces': int(len(T)), 'flip_after_vs_fit': int(flip_after_fit.sum()), 'flip_after_vs_donor': int(flip_after_donor.sum()), 'flip_fit_vs_donor': int(flip_fit_donor.sum()),
               'per_state': {k: {'inverted_local': int(metrics[k]['inverted_local'].sum()), 'folded_edge_gt150': int(metrics[k]['folded_edge'].sum()), 'sliver_lt3deg': int(metrics[k]['sliver'].sum()), 'collapsed_vs_donor': int(collapsed[k].sum()) if k in collapsed else 0, 'self_intersecting_pairs': len(inter[k]), 'self_intersecting_faces': len(inter_faces[k]), 'min_area_mm2': round(float(areas[k].min() * 1e6), 4), 'min_angle_deg': round(float(metrics[k]['min_angle_deg'].min()), 3)} for k in states},
               'classes': {c: count(lambda r, c=c: r['class'] == c) for c in ('introduced', 'inherited_donor', 'inherited_fit', 'resolved', 'sign_only')},
               'introduced_by_region': {}, 'after_defects_by_region': {}, 'introduced_by_shell': {}}
    for r in records:
        if r['class'] == 'introduced':
            summary['introduced_by_region'][r['region']] = summary['introduced_by_region'].get(r['region'], 0) + 1
            key = r['shell'] + ':' + '+'.join(r['defects']['after']); summary['introduced_by_shell'][key] = summary['introduced_by_shell'].get(key, 0) + 1
        if r['defects']['after']: summary['after_defects_by_region'][r['region']] = summary['after_defects_by_region'].get(r['region'], 0) + 1
    summary['after_skin_defects'] = {'faces': [r['face'] for r in records if r['shell'] == 'skin' and r['defects']['after']], 'introduced_faces': [r['face'] for r in records if r['shell'] == 'skin' and r['class'] == 'introduced']}

    def pair_classes(k):
        c = {}
        for i, j in inter[k]: key = '/'.join(sorted((str(fcls[i]), str(fcls[j])))); c[key] = c.get(key, 0) + 1
        return c

    new_pairs = sorted(set(inter['after']) - set(inter['fit'])); gone_pairs = sorted(set(inter['fit']) - set(inter['after']))
    skin_new = [(i, j) for i, j in new_pairs if fcls[i] == 'skin' and fcls[j] == 'skin']
    summary['self_intersections_by_class'] = {k: pair_classes(k) for k in states}
    summary['new_pairs_vs_fit'] = {'total': len(new_pairs), 'skin_skin': len(skin_new), 'skin_skin_faces': sorted({f for p in skin_new for f in p}), 'skin_skin_regions': sorted({region_of_face(f) for p in skin_new for f in p})}
    summary['gone_pairs_vs_fit'] = len(gone_pairs)
    summary['coverage_limits'] = 'exact triangle-triangle tests on non-adjacent bounding-box candidates; coplanar overlaps and adjacent (edge/vertex-sharing) contacts are NOT detected; fold/dihedral/sliver are screening metrics'
    lists = {'introduced_skin': summary['after_skin_defects']['introduced_faces'], 'skin_skin_new_intersections': summary['new_pairs_vs_fit']['skin_skin_faces'], 'flip_after_vs_fit': np.flatnonzero(flip_after_fit).tolist(), 'introduced': [r['face'] for r in records if r['class'] == 'introduced'],
             'inherited': [r['face'] for r in records if r['class'].startswith('inherited')], 'resolved': [r['face'] for r in records if r['class'] == 'resolved'], 'sign_only': [r['face'] for r in records if r['class'] == 'sign_only'],
             'after_self_intersect': sorted(inter_faces['after'].keys()), 'fit_self_intersect': sorted(inter_faces['fit'].keys()), 'donor_self_intersect': sorted(inter_faces['donor'].keys()),
             'after_inverted_local': np.flatnonzero(metrics['after']['inverted_local']).tolist(), 'fit_inverted_local': np.flatnonzero(metrics['fit']['inverted_local']).tolist(), 'donor_inverted_local': np.flatnonzero(metrics['donor']['inverted_local']).tolist()}
    return {'summary': summary, 'records': records, 'lists': lists, 'pairs': {k: [[int(a), int(b)] for a, b in v] for k, v in inter.items()}, 'face_shell': fcls.tolist()}


def collision_triggers(records):
    """Fit-introduced skin faces that self-intersect in the 'after' state: the collision-repair trigger set
    (derived from classification, never from vertex ids)."""
    return sorted(int(r['face']) for r in records if r['shell'] == 'skin' and r['defects']['fit'] and not r['defects']['donor'] and 'self_intersect' in r['defects']['after'])
