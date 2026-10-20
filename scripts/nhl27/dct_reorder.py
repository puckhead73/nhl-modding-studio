"""Losslessly reorder whole DCT channels without requantizing animation samples.

Packed scalar lanes must already have the same order. This intentionally rejects
lane repacking, missing DOFs, and crossings between rotation/vector categories.
"""
import struct
from animation_dct import Bits, parse


def permutation(source, target, quaternions, vectors, float_vectors):
    if len(source)!=len(target) or len(set(source))!=len(source) or set(source)!=set(target):
        raise ValueError('DOF bindings are not a unique bijection')
    qv=quaternions+vectors
    if not qv<=len(source)<=qv+4*float_vectors or (len(source)-qv+3)//4!=float_vectors:
        raise ValueError('Binding count does not match channel categories')
    order=[]
    for lo,hi in [(0,quaternions),(quaternions,qv)]:
        if set(source[lo:hi])!=set(target[lo:hi]):raise ValueError('DOF category mismatch')
        lookup={x:i for i,x in enumerate(source)}
        order.extend(lookup[x] for x in target[lo:hi])
    if source[qv:]!=target[qv:]:raise ValueError('Scalar lane repacking is not qualified')
    return order+list(range(qv,qv+float_vectors))


def reorder(data, order):
    meta,stream=parse(data);channels=meta['channels'];n=len(channels)
    if sorted(order)!=list(range(n)):raise ValueError('Channel order is not a permutation')
    q=meta['quaternions'];v=meta['vectors']
    for lo,hi in [(0,q),(q,q+v),(q+v,n)]:
        if set(order[lo:hi])!=set(range(lo,hi)):raise ValueError('Channel category crossing')
    body=struct.pack('<5H2B',meta['keys'],q,v,meta['float_vectors'],
                     meta['quantizer'],meta['subblock_quantizer'],meta['escape_bits'])
    body+=bytes(len(channels[i]['widths'])<<4 for i in order)
    body+=bytes(len(body)%2)
    for i in order:
        c=channels[i];body+=struct.pack('<4h',*c['base'])
        for widths in c['widths']:
            word=0
            for width in widths:
                if width>=15:
                    if width!=meta['escape_bits']:raise ValueError('Width cannot use escape code')
                    width=15
                word=(word<<4)|width
            body+=struct.pack('<H',word)
    reader=Bits(stream);out=bytearray((meta['stream_bits']+127)//128*16+16);position=0
    for block in range((meta['keys']+7)//8):
        rows=[]
        for c in channels:
            values=[]
            for k,widths in enumerate(c['widths']):
                if block==0 and k==0:continue
                for width in widths:values.append((reader.signed(width),width))
            rows.append(values)
        for i in order:
            for value,width in rows[i]:
                for bit in range(width):
                    out[position//8]|=((value>>bit)&1)<<(position%8);position+=1
    if reader.position!=meta['stream_bits'] or position!=reader.position:
        raise ValueError('Reordered bit budget mismatch')
    body+=out;keys=meta['envelope']['key_times'];kt=struct.pack('<'+'H'*len(keys),*keys)
    result=struct.pack('<5I',len(keys),32,0,len(body),32+len(kt))+bytes(12)+kt+body
    return result+bytes((-len(result))%16)+struct.pack('<II',4,16)
