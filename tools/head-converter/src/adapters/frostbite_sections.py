"""Bounded NHL26 observed-layout index decoder; not a generic Frostbite decoder.

Caller supplies measured declaration metadata. The index region follows the
16-byte-aligned combined section vertex payload; startIndex is in u16 elements.
"""
import struct

def decode_indices(geometry, sections):
    end=0
    for s in sections:
        count, size, base = s['vertexCount'], s['vertex_bytes'], s['startVertex']
        if count <= 0 or size <= 0 or base != end:
            raise ValueError('noncontiguous/invalid observed vertex layout')
        end += size
    index_base=(end+15)&~15
    result=[]
    for s in sections:
        count=s['primitiveCount']*3
        if count < 0 or s['startIndex'] < 0:
            raise ValueError('negative section index extent')
        start=index_base+2*s['startIndex']
        if start+count*2 > len(geometry):
            raise ValueError('section index extent exceeds geometry chunk')
        ix=struct.unpack_from('<'+'H'*count,geometry,start)
        if any(i>=s['vertexCount'] for i in ix):
            raise ValueError('section index exceeds local vertex count')
        result.append([list(ix[i:i+3]) for i in range(0,count,3)])
    return index_base,result
