"""Bounded NHL27 DCT resource reader and experimental neutral motion export.

Format cross-check: marv7000/AssetBankPlugin, Ant/DCT (fd88e55f).
Uses the mathematical 8-point inverse DCT, not the engine's rounded table.
Outputs unbound channels, never claims a target-ready ANT asset.
"""
import argparse
import hashlib
import json
import math
import struct
from pathlib import Path


def take(data,at,size):
    if at<0 or size<0 or at+size>len(data):raise ValueError('Animation range outside resource')
    return data[at:at+size]


def envelope(data):
    count,keys_at,zero,size,body_at=struct.unpack('<5I',take(data,0,20))
    if keys_at!=32 or zero or any(take(data,20,12)) or body_at!=32+2*count:
        raise ValueError('Unsupported animation resource envelope')
    keys=list(struct.unpack('<'+'H'*count,take(data,keys_at,count*2)))
    if keys!=sorted(set(keys)):raise ValueError('Key times are not strictly increasing')
    body=take(data,body_at,size);tail=data[body_at+size:]
    padding=(-(body_at+size))%16
    if tail!=bytes(padding)+struct.pack('<II',4,16):raise ValueError('Unrecognized resource trailer')
    return dict(key_times=keys,body_offset=body_at,body_size=size,padding=padding),body


def parse(data):
    env,b=envelope(data)
    keys,quats,vecs,floats,quant,sub,catch=struct.unpack('<5H2B',take(b,0,12))
    count=quats+vecs+floats
    if not 0<keys<=65535 or not 0<count<=4096 or not quant or not 0<catch<=16:
        raise ValueError('Invalid DCT dimensions/quantizer')
    if env['key_times'] and len(env['key_times'])!=keys:raise ValueError('DCT/envelope key count mismatch')
    descriptors=take(b,12,count);p=(12+count+1)&~1;channels=[]
    if any(take(b,12+count,p-12-count)):raise ValueError('Nonzero descriptor padding')
    for desc in descriptors:
        n=desc>>4
        if n>8 or desc&15:raise ValueError('Unsupported DCT channel descriptor')
        base=list(struct.unpack('<4h',take(b,p,8)));p+=8
        words=struct.unpack('<'+'H'*n,take(b,p,n*2));p+=n*2
        widths=[[catch if ((w>>shift)&15)==15 else ((w>>shift)&15)
            for shift in (12,8,4,0)] for w in words]
        channels.append(dict(base=base,widths=widths))
    bits=sum(sum(sum(ws) for ws in c['widths'][1:]) for c in channels)
    bits+=((keys+7)//8-1)*sum(sum(sum(ws) for ws in c['widths']) for c in channels)
    stream=b[p:]
    # Observed DCT buffers include a 16-byte zero read-ahead guard after 16-byte rounding.
    encoded=(bits+127)//128*16
    if len(stream)!=encoded+16 or any(stream[encoded:]):raise ValueError(f'DCT bit budget mismatch: {bits} bits / {len(stream)} bytes')
    return dict(envelope=env,keys=keys,quaternions=quats,vectors=vecs,float_vectors=floats,
        quantizer=quant,subblock_quantizer=sub,escape_bits=catch,channels=channels,
        stream_offset=env['body_offset']+p,stream_bits=bits),stream


class Bits:
    def __init__(self,data):self.data=data;self.position=0
    def signed(self,width):
        if not 0<=width<=16 or self.position+width>len(self.data)*8:raise ValueError('Bitstream overrun')
        v=0
        # NHL PS5 uses least-significant bit first for both byte and value.
        for bit in range(width):
            v|=((self.data[self.position//8]>>(self.position%8))&1)<<bit
            self.position+=1
        return v-(1<<width) if width and v&(1<<(width-1)) else v


def decode(data):
    meta,stream=parse(data);reader=Bits(stream);frames=[]
    if meta['keys']*len(meta['channels'])>2_000_000:raise ValueError('Motion export sample budget exceeded')
    # Channel coefficients are independent. Constant channels carry DC only.
    weights=[[((.25 if k==0 else .5*math.cos(math.pi*(2*t+1)*k/16)) *
        (1+meta['subblock_quantizer']*.1*k)/meta['quantizer']) for k in range(8)] for t in range(8)]
    for block in range((meta['keys']+7)//8):
        coefficients=[]
        for channel in meta['channels']:
            coef=[[0]*4 for _ in range(8)]
            for k,widths in enumerate(channel['widths']):
                if block==0 and k==0:continue
                coef[k]=[reader.signed(w) for w in widths]
            # Engine coefficient storage is signed 16-bit.
            coef[0]=[((a+b+32768)%65536)-32768 for a,b in zip(coef[0],channel['base'])]
            coefficients.append(coef)
        for t in range(min(8,meta['keys']-block*8)):
            frames.append([[sum(weights[t][k]*c[k][axis] for k in range(8))
                for axis in range(4)] for c in coefficients])
    if reader.position!=meta['stream_bits']:raise ValueError('Decoder did not consume predicted bits')
    norms=[math.sqrt(sum(v*v for v in c)) for frame in frames for c in frame[:meta['quaternions']]]
    if not all(math.isfinite(x) for frame in frames for c in frame for x in c):raise ValueError('Non-finite decoded motion')
    error=max((abs(x-1) for x in norms),default=0)
    if error>.02:raise ValueError(f'Quaternion sanity check failed: {error}')
    return meta,frames,dict(min_quaternion_norm=min(norms,default=None),max_quaternion_norm=max(norms,default=None),
        max_quaternion_norm_error=error if norms else None,consumed_bits=reader.position)


def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('resource',type=Path);ap.add_argument('output',type=Path)
    a=ap.parse_args();data=a.resource.read_bytes();meta,frames,checks=decode(data)
    report=dict(status='experimental_unbound_motion',source_sha256=hashlib.sha256(data).hexdigest(),
        metadata=meta,checks=checks,frames=frames,
        limitations=['No skeleton/channel-name binding','No target ANT serialization','Engine float parity unqualified'])
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report))
    print(json.dumps(dict(keys=meta['keys'],channels=len(meta['channels']),checks=checks),indent=2))


if __name__=='__main__':main()
