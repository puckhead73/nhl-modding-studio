"""Bounded VBR coefficient stream reader/writer; no game code execution."""
import struct


def decode(widths, block):
    if len(block)<3:raise ValueError('Missing VBR quantizers')
    bits=int.from_bytes(block[3:],'little');position=0;limit=(len(block)-3)*8
    def read(n):
        nonlocal position
        if position+n>limit:raise ValueError('Truncated VBR coefficients')
        value=(bits>>position)&((1<<n)-1);position+=n
        return value
    channels=[]
    for word in widths:
        if not 0<=word<=0xffffffff:raise ValueError('Invalid width descriptor')
        sizes=[(word>>(4*k))&15 for k in range(8)]
        present=[read(1) if n else 0 for n in sizes]
        signs=[read(1) if p else 0 for p in present]
        channels.append([(1 if sign else -1)*read(n) if p else 0
                         for n,p,sign in zip(sizes,present,signs)])
    if limit-position>7 or bits>>position:raise ValueError('Unexpected coefficient padding')
    return channels


def encode(widths, channels, quantizers):
    if len(widths)!=len(channels) or len(quantizers)!=3:raise ValueError('Invalid block dimensions')
    bits=0;position=0
    def write(value,n):
        nonlocal bits,position
        if not 0<=value<(1<<n):raise ValueError('Coefficient exceeds width')
        bits|=value<<position;position+=n
    for word,values in zip(widths,channels):
        if not 0<=word<=0xffffffff or len(values)!=8:raise ValueError('Invalid coefficient descriptor')
        sizes=[(word>>(4*k))&15 for k in range(8)]
        for n,v in zip(sizes,values):
            if n:write(int(v!=0),1)
            elif v:raise ValueError('Nonzero coefficient with zero width')
        for v in values:
            if v:write(int(v>0),1)
        for n,v in zip(sizes,values):
            if v:write(abs(v),n)
    return bytes(quantizers)+bits.to_bytes((position+7)//8,'little')


def padded(channels,q,v):
    count=len(channels)
    if count<q*4+v*3:raise ValueError('Invalid channel dimensions')
    stride=4*(q+v)+((count-4*q-3*v+3)&~3);output=[0]*(stride*8)
    for j,values in enumerate(channels):
        pad=0 if j<4*q else min(v,(j-4*q)//3)
        index=j+pad;start=(index&3)+(index&~3)*8
        for k,value in enumerate(values):output[start+4*k]=value
    return struct.pack('<'+'i'*len(output),*output),stride
