"""Mouth-interior UV-island segmentation of the occluded mouth component (numpy only).

Port of the island part of experiments/claude/measure/mouth_lining_diag.py: the occluded (non outer-shell) faces of
the donor are split into connected components (seam groups merged); the one holding the oral ids is the mouth; its
raw-id connected pieces (UV seams split them) are the islands, named by geometry on the NHL atlas-A donors
(teeth/palate, lower teeth, tongue, bag rear/sides, inner upper/lower lip lining, corners). Names starting with
'inner_' are the lip lining used by the collision repair.
"""
import collections
import numpy as np
from . import geometry as G, face_quality as FQ


def components(faces, tri):
    parent = {int(f): int(f) for f in faces}

    def find(a):
        while parent[a] != a: parent[a] = parent[parent[a]]; a = parent[a]
        return a
    edge = {}
    for f in faces:
        a, b, c = tri[f]
        for e in ((a, b), (b, c), (c, a)):
            k = (min(e), max(e))
            if k in edge: parent[find(edge[k])] = find(int(f))
            else: edge[k] = int(f)
    comp = collections.defaultdict(list)
    for f in faces: comp[find(int(f))].append(int(f))
    return list(comp.values())


def islands(donor, T, oral_ids, outer_faces=None):
    """{island name: [face ids]} for the donor head."""
    n = len(donor); oral = np.zeros(n, bool); oral[np.asarray(list(oral_ids), int)] = True
    outer = FQ.outer_shell_faces(donor, T) if outer_faces is None else np.asarray(outer_faces, bool); interior = np.flatnonzero(~outer)
    dup = G.seam_groups(donor); canon = np.arange(n)
    for ids in dup:
        for i in ids: canon[i] = ids[0]
    mouth = max(components(interior, canon[T]), key=lambda fs: oral[np.unique(T[fs])].sum())
    isl = components(np.array(mouth), T)

    def name_island(fs):
        v = np.unique(T[fs]); c = donor[v].mean(0); nrm = FQ.face_normals(donor, T[fs]).mean(0); frac = oral[v].mean()
        if frac > 0.5:
            if abs(nrm[0]) > 0.6: return 'bag_side_' + ('L' if c[0] > 0 else 'R')
            if nrm[2] < -0.25 and c[1] < -0.09: return 'upper_teeth_palate'
            if nrm[2] > 0.25 and c[1] < -0.09: return 'lower_teeth'
            if c[1] > -0.07 and nrm[2] < 0: return 'upper_bag_rear'
            return 'tongue'
        if len(v) < 30: return 'inner_lip_corner_' + ('L' if c[0] > 0 else 'R')
        return 'inner_upper_lip' if nrm[2] < 0 else 'inner_lower_lip'

    named = {}
    for fs in sorted(isl, key=len, reverse=True):
        nm = name_island(fs); k = nm; i = 2
        while k in named: k = f'{nm}_{i}'; i += 1
        named[k] = [int(f) for f in fs]
    return named


def lining_faces(named):
    return np.array(sorted({f for k, fs in named.items() if k.startswith('inner_') for f in fs}), int)
