"""Bounded GLB2 inspector. Retains raw chunks; refuses unsupported accessor layouts.

Inspection only: mesh coordinates are reported in accessor space. No rig inference,
UV flips, topology welding or native-format support is implied by this reader.
"""
from collections import Counter
from pathlib import Path
import hashlib
import json
import math
import struct

FORMATS = {5120: ('b', 1), 5121: ('B', 1), 5122: ('h', 2), 5123: ('H', 2), 5125: ('I', 4), 5126: ('f', 4)}
COMPONENTS = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}

def signature(values):
    return hashlib.sha256(json.dumps(values, separators=(',', ':'), allow_nan=False).encode()).hexdigest()

class Glb:
    def __init__(self, data):
        self.data = data
        if len(data) < 20:
            raise ValueError('truncated GLB')
        magic, version, total = struct.unpack_from('<4sII', data)
        if magic != b'glTF' or version != 2 or total != len(data):
            raise ValueError('invalid GLB2 header/length')
        self.chunks = []
        offset = 12
        while offset < total:
            if offset + 8 > total:
                raise ValueError('truncated chunk header')
            size, kind = struct.unpack_from('<I4s', data, offset)
            offset += 8
            if size % 4 or offset + size > total:
                raise ValueError('invalid chunk extent/alignment')
            self.chunks.append((kind, data[offset:offset+size]))
            offset += size
        if self.chunks[0][0] != b'JSON':
            raise ValueError('JSON must be first')
        self.doc = json.loads(self.chunks[0][1])
        bins = [b for t, b in self.chunks if t == b'BIN\0']
        if len(bins) > 1 or len(self.doc.get('buffers', [])) > 1:
            raise ValueError('multiple buffers unsupported')
        self.blob = bins[0] if bins else b''
        for b in self.doc.get('buffers', []):
            if 'uri' in b or b['byteLength'] > len(self.blob):
                raise ValueError('external or truncated buffer')

    def serialize(self):
        # Unknown chunks and JSON padding survive the unchanged round trip.
        return struct.pack('<4sII', b'glTF', 2, len(self.data)) + b''.join(
            struct.pack('<I4s', len(b), t) + b for t, b in self.chunks)

    def accessor(self, index):
        a = self.doc['accessors'][index]
        if 'sparse' in a or 'bufferView' not in a:
            raise ValueError('sparse/implicit accessor unsupported')
        if a['type'] not in COMPONENTS or a['componentType'] not in FORMATS:
            raise ValueError('unsupported accessor type')
        fmt, size = FORMATS[a['componentType']]
        n = COMPONENTS[a['type']]
        if a['type'] == 'MAT4' and a['componentType'] != 5126:
            raise ValueError('only float MAT4 supported')
        view = self.doc['bufferViews'][a['bufferView']]
        if view.get('buffer', 0) != 0:
            raise ValueError('external buffer')
        stride = view.get('byteStride', n * size)
        start = view.get('byteOffset', 0)
        rel = a.get('byteOffset', 0)
        count = a['count']
        end = rel + ((count - 1) * stride + n * size if count else 0)
        if min(start, rel, count, view['byteLength']) < 0 or stride < n*size or stride % size or end > view['byteLength'] or start+view['byteLength'] > len(self.blob):
            raise ValueError('accessor outside bufferView or invalid stride')
        rows = [struct.unpack_from('<' + fmt*n, self.blob, start+rel+i*stride) for i in range(count)]
        if any(not math.isfinite(v) for row in rows for v in row):
            raise ValueError('nonfinite accessor')
        if a.get('normalized') and fmt != 'f':
            signed = fmt in ('b', 'h')
            denom = (1 << (size*8 - int(signed))) - 1
            rows = [tuple(max(-1.0, v/denom) for v in row) for row in rows]
        return rows

    def primitives(self):
        for mi, mesh in enumerate(self.doc.get('meshes', [])):
            for pi, prim in enumerate(mesh['primitives']):
                attrs = {k: self.accessor(v) for k, v in prim['attributes'].items()}
                pos = attrs.get('POSITION', [])
                if any(len(v) != len(pos) for v in attrs.values()):
                    raise ValueError('attribute counts differ')
                indices = [v[0] for v in self.accessor(prim['indices'])] if 'indices' in prim else list(range(len(pos)))
                if prim.get('mode', 4) != 4 or len(indices) % 3:
                    raise ValueError('only triangle lists supported')
                if any(not isinstance(v, int) or v < 0 or v >= len(pos) for v in indices):
                    raise ValueError('invalid triangle index')
                yield mi, pi, prim, attrs, indices

    def report(self):
        report = dict(sha256=hashlib.sha256(self.data).hexdigest(), bytes=len(self.data),
                      unchanged_chunk_roundtrip=self.serialize() == self.data, meshes=[], skins=[],
                      generator=self.doc.get('asset', {}).get('generator'),
                      limitations=['accessor-space geometry; no scene transform or animation parity claim'])
        for mi, pi, prim, attrs, ix in self.primitives():
            pos = attrs['POSITION']
            tris = [ix[i:i+3] for i in range(0, len(ix), 3)]
            edges = Counter(tuple(sorted((t[k], t[(k+1)%3]))) for t in tris for k in range(3))
            uv = attrs.get('TEXCOORD_0')
            q = [[round(u*2048), round(v*2048)] for u, v in uv] if uv else None
            p = dict(mesh=mi, primitive=pi, material=prim.get('material'), vertices=len(pos), triangles=len(tris),
                     attributes=list(attrs), topology_ordered_sha256=signature(ix),
                     position_ordered_sha256=signature(pos),
                     bbox=[[min(v[k] for v in pos), max(v[k] for v in pos)] for k in range(3)],
                     boundary_edges=sum(n==1 for n in edges.values()), nonmanifold_edges=sum(n>2 for n in edges.values()),
                     repeated_index_triangles=sum(len(set(t))<3 for t in tris),
                     exact_unique_positions=len(set(pos)), morph_targets=len(prim.get('targets', [])))
            if q:
                p.update(uv_ordered_sha256=signature(q), uv_multiset_sha256=signature(sorted(q)),
                         uv_corner_sha256=signature([q[i] for i in ix]), uv_unique_2048=len(set(map(tuple,q))),
                         uv_bbox=[[min(v[k] for v in uv), max(v[k] for v in uv)] for k in range(2)])
            joints, weights = [], []
            sets = sorted(k[7:] for k in attrs if k.startswith('JOINTS_'))
            for s in sets:
                if 'WEIGHTS_'+s not in attrs:
                    raise ValueError('unpaired skin attributes')
            if sets:
                for i in range(len(pos)):
                    joints.append(sum((list(attrs['JOINTS_'+s][i]) for s in sets), []))
                    weights.append(sum((list(attrs['WEIGHTS_'+s][i]) for s in sets), []))
                mass = Counter()
                for js, ws in zip(joints, weights):
                    for j, w in zip(js, ws):
                        if w < 0:
                            raise ValueError('negative weight')
                        if w > 0:
                            mass[j] += w
                p['skin'] = dict(max_influences=max(sum(w>0 for w in ws) for ws in weights),
                                 max_sum_error=max(abs(sum(ws)-1) for ws in weights),
                                 active_joint_mass=dict(sorted(mass.items())))
            report['meshes'].append(p)
        nodes = self.doc.get('nodes', [])
        for skin in self.doc.get('skins', []):
            js = skin['joints']
            names = [nodes[j].get('name', '') for j in js]
            joint_set = set(js)
            links = [(i, child) for i in js for child in nodes[i].get('children', []) if child in joint_set]
            ib = self.accessor(skin['inverseBindMatrices']) if 'inverseBindMatrices' in skin else []
            report['skins'].append(dict(joints=len(js), parent_links=len(links),
                duplicate_names={n:c for n,c in Counter(names).items() if c>1},
                unnamed=sum(not n or n.startswith('bone_') for n in names),
                inverse_binds=len(ib), inverse_bind_sha256=signature(ib),
                nonidentity_rotations=sum(n.get('rotation',[0,0,0,1]) != [0,0,0,1] for n in (nodes[j] for j in js)),
                original_nodes_sha256=signature([nodes[j] for j in js])))
        return report

def inspect_file(path):
    p = Path(path)
    return dict(path=str(p), **Glb(p.read_bytes()).report())
