"""Reusable shared-UV correspondence matcher, extracted from the successful NHL19 test.

Run in Blender: --background --factory-startup --python-exit-code 1 --python
uv_template_match.py -- input.json output.json

Input: donor/source each contain positions, uv and triangles arrays in a canonical
UV convention. Positions must use metres for the default ambiguity threshold.
Source records may be position/UV-deduplicated; never weld/reorder native donor IDs.
Produces correspondence only, not native packing or an automatic unsupported-region fit.
"""
import hashlib
import json
import math
import sys
from mathutils import Vector
from mathutils.kdtree import KDTree


def validate(mesh):
    positions, uv, triangles = mesh['positions'], mesh['uv'], mesh['triangles']
    if not positions or len(positions) != len(uv):
        raise ValueError('Nonempty, one-UV-per-record arrays required; preserve split records')
    for values, width in ((positions, 3), (uv, 2)):
        if any(len(v) != width or not all(math.isfinite(x) for x in v) for v in values):
            raise ValueError('Malformed or non-finite mesh attributes')
    if not triangles or any(len(t) != 3 or any(type(i) is not int or not 0 <= i < len(positions) for i in t) for t in triangles):
        raise ValueError('Triangle indices invalid')


def match_templates(donor, source, uv_tolerance=.0005, tie_epsilon=1e-7,
                    position_ambiguity=.0001):
    """Return native donor index -> source record index with shared-edge evidence.

    Thresholds are the NHL19 experimental defaults, not universal game constants.
    UV ties that point at different surfaces are rejected rather than guessed.
    """
    validate(donor)
    validate(source)
    if any(not math.isfinite(x) or x < 0 for x in (uv_tolerance, tie_epsilon, position_ambiguity)):
        raise ValueError('Invalid matching thresholds')
    tree = KDTree(len(source['uv']))
    for i, uv in enumerate(source['uv']):
        tree.insert((uv[0], uv[1], 0), i)
    tree.balance()
    points = [Vector(p) for p in source['positions']]
    matches, distances, ambiguous = {}, {}, []
    for i, uv in enumerate(donor['uv']):
        query = (uv[0], uv[1], 0)
        distance = tree.find(query)[2]
        if distance > uv_tolerance:
            continue
        candidates = tree.find_range(query, distance + tie_epsilon)
        if not candidates:
            continue
        candidates.sort(key=lambda row: row[2])
        sid = candidates[0][1]
        if max((points[j]-points[sid]).length for _, j, _ in candidates) > position_ambiguity:
            ambiguous.append(i)
            continue
        matches[i], distances[i] = sid, candidates[0][2]
    source_faces = {tuple(sorted(t)) for t in source['triangles']}
    source_edges = {tuple(sorted((a, b))) for t in source['triangles']
                    for a, b in zip(t, t[1:]+t[:1])}
    neighbors = [set() for _ in donor['positions']]
    shared_faces, different_faces = [], []
    for face in donor['triangles']:
        for a, b in zip(face, face[1:]+face[:1]):
            neighbors[a].add(b)
            neighbors[b].add(a)
        if all(i in matches for i in face):
            bucket = shared_faces if tuple(sorted(matches[i] for i in face)) in source_faces else different_faces
            bucket.append(face)
    edge_supported = {i for i, sid in matches.items() if sum(
        j in matches and tuple(sorted((sid, matches[j]))) in source_edges
        for j in neighbors[i]) >= 2}
    trusted = edge_supported | {i for face in shared_faces for i in face}
    return {
        'method': 'shared_uv_template_v1',
        'thresholds': {'uv': uv_tolerance, 'uv_tie': tie_epsilon, 'source_position_ambiguity_m': position_ambiguity},
        'uv_matches': matches,
        'mapping': {i: matches[i] for i in sorted(trusted)},
        'ambiguous': ambiguous,
        'unsupported': sorted(set(range(len(donor['positions'])))-trusted),
        'shared_triangles': len(shared_faces),
        'fully_matched_nonshared_triangles': len(different_faces),
        'shared_edge_supported_vertices': len(edge_supported),
        'supported_fraction': len(trusted)/len(donor['positions']),
        'max_uv_error': max(distances.values(), default=0),
        'qualification': 'correspondence evidence only; requires template/region and target-format validation',
    }


if __name__ == '__main__':
    args = sys.argv[sys.argv.index('--')+1:]
    if len(args) != 2:
        raise RuntimeError('Expected input.json output.json after --')
    with open(args[0], 'rb') as f:
        raw = f.read()
    data = json.loads(raw)
    result = match_templates(data['donor'], data['source'], **data.get('options', {}))
    result['input_sha256'] = hashlib.sha256(raw).hexdigest()
    with open(args[1], 'w') as f:
        json.dump(result, f, indent=2)
    print('MAPPED', len(result['mapping']), 'UNSUPPORTED', len(result['unsupported']))
