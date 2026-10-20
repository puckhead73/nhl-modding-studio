"""Decode staged VBR dynamic blocks with the audited, build-bound native oracle.

This checks compressed motion, not graph execution or gameplay parity. Run in a
disposable process because the oracle executes isolated native codec functions.
"""
import hashlib
import json
import math
import struct
import sys
from pathlib import Path
from animation_dct import take
from vbr_stage import envelope
from vbr_oracle import Oracle
import vbr_coefficients


def layout(fields, data):
    _, palette, keywords, stream = envelope(data)
    q, v, f = (fields[k] for k in ('QuaternionCount','Vector3Count','FloatCount'))
    cq, cv, cf = (fields[k] for k in ('ConstQuaternionCount','ConstVector3Count','ConstFloatCount'))
    pos = fields['KeyTimeSize']
    times = take(stream, 0, pos)
    count = cq*4+cv*3+cf
    width = 1 if fields['ConstPaletteSize'] <= 256 else 2
    indices = list(struct.unpack('<'+('B' if width == 1 else 'H')*count, take(stream,pos,count*width)))
    if any(i >= fields['ConstPaletteSize'] for i in indices):
        raise ValueError('Constant palette index outside bounds')
    pos += count*width
    runs = list(take(stream,pos,fields['ConstChanMapSize']));pos += len(runs)
    if sum(runs[::2]) != q+v+f or sum(runs[1::2]) != cq+cv+cf:
        raise ValueError('Unsupported channel run map')
    widths = list(struct.unpack('<'+'I'*(q*4+v*3+f),take(stream,pos,4*(q*4+v*3+f))))
    pos += 4*len(widths)
    if fields['VectorOffsetSize'] or fields['FloatOffsetSize']:
        raise ValueError('Offset tracks require additional reconstruction')
    sizes = [x[0] for x in struct.iter_unpack('<H', keywords)]
    if len(sizes) != (fields['NumKeys']+7)//8:
        raise ValueError('Unexpected compressed block count')
    blocks=[]
    for size in sizes:
        blocks.append(take(stream,pos,size));pos += size
    tail=stream[pos:]
    if len(tail)>16 or any(tail):raise ValueError('Unexpected VBR stream trailer')
    return dict(q=q,v=v,f=f,times=list(times),runs=runs,indices=indices,
                widths=widths,blocks=blocks,palette=palette)


def run(root):
    rows=json.loads((root/'deke-trial/vbr/stage.json').read_text());results=[]
    oracle=Oracle(root.parent)
    target=Oracle(Path('local/nhl24-eboot/update'),target=True)
    try:
        for row in rows:
            result=dict(name=row['name'],layer=row['layer'])
            try:
                prefix=Path(row['prefix'])
                instance=next(x for x in row['fields']['instances'] if 'VbrAnimationAsset' in x['type'])
                fields=instance['fields'];parts=layout(fields,prefix.with_suffix('.resource').read_bytes())
                asset=take(prefix.with_suffix('.ebx').read_bytes(),32+instance['offset'],168)
                target_asset=bytearray(184)
                target_asset[64:134]=asset[96:166]
                decoded=[];coefficients=[]
                for block in parts['blocks']:
                    coeff,stride=oracle.unpack(parts['q'],parts['v'],parts['widths'],block)
                    portable=vbr_coefficients.decode(parts['widths'],block)
                    if vbr_coefficients.padded(portable,parts['q'],parts['v'])!=(coeff,stride):
                        raise ValueError('Portable coefficient decoder differs')
                    if vbr_coefficients.encode(parts['widths'],portable,block[:3])!=block:
                        raise ValueError('Coefficient round trip is not byte-exact')
                    samples=oracle.inverse(asset,coeff,stride,list(block[:3]))
                    tc,ts=target.unpack(parts['q'],parts['v'],parts['widths'],block)
                    if tc!=coeff or ts!=stride:raise ValueError('Target coefficient decoder differs')
                    target_samples=target.inverse(target_asset,tc,ts,list(block[:3]))
                    if target_samples!=samples:raise ValueError('Target inverse transform differs')
                    values=[x[0] for x in struct.iter_unpack('<f',samples)]
                    if not all(math.isfinite(x) for x in values):raise ValueError('Non-finite decoded motion')
                    coefficients.append(coeff);decoded.append(samples)
                payload=b''.join(decoded)
                prefix.with_suffix('.dynamic.f32').write_bytes(payload)
                result.update(status='dynamic_blocks_bit_exact_across_engines',blocks=len(decoded),stride=stride,
                    portable_coefficients_match=True,coefficient_repack_byte_exact=True,
                    decoded_sha256=hashlib.sha256(payload).hexdigest(),
                    coefficient_sha256=hashlib.sha256(b''.join(coefficients)).hexdigest(),
                    max_abs=max(abs(x[0]) for x in struct.iter_unpack('<f',payload)))
            except (ValueError,KeyError,StopIteration) as exc:
                result.update(status='rejected',error=str(exc))
            results.append(result)
    finally:oracle.close();target.close()
    (root/'deke-trial/vbr/decode-audit.json').write_text(json.dumps(results,indent=2))
    print(json.dumps(results,indent=2))


if __name__=='__main__':run(Path(sys.argv[1]))
