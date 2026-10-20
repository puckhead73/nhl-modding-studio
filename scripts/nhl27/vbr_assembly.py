"""Assemble VBR keyed tracks for inspection, independently of runtime graph state.

Quaternion normalization here uses Python arithmetic; it is a comparison aid,
not a claim of bit-identical engine interpolation or final skeleton evaluation.
"""
import math
import struct
from vbr_remap import tracks


def key_times(encoded,num_keys,end_frame):
    runs=[];i=0
    while i<len(encoded):
        value=encoded[i];i+=1
        if value&128:
            if i==len(encoded):raise ValueError('Truncated key-time run')
            value=((value&127)<<8)|encoded[i];i+=1
        runs.append(value)
    result=[];frame=0
    for i,count in enumerate(runs):
        if not i%2:result.extend(range(frame,frame+count))
        frame+=count
    if len(result)!=num_keys or not result or result[-1]!=end_frame:
        raise ValueError('Key-time coverage mismatch')
    return result


def assemble(fields,parts,decoded,stride):
    channels=tracks(fields,parts);palette=list(struct.unpack('<'+'f'*(len(parts['palette'])//4),parts['palette']))
    times=key_times(parts['times'],fields['NumKeys'],fields['EndFrame'])
    samples=[struct.unpack('<'+'f'*(len(b)//4),b) for b in decoded];frames=[]
    q,v,f=parts['q'],parts['v'],parts['f']
    for key in range(fields['NumKeys']):
        block=key//8
        row=key%8 if key<8 or key<(fields['NumKeys']&~7) else key-fields['NumKeys']+8
        vector=samples[block][row*stride:(row+1)*stride]
        dynamic=[list(vector[i*4:i*4+4]) for i in range(q)]
        dynamic += [list(vector[4*q+i*4:4*q+i*4+3]) for i in range(v)]
        dynamic += [[vector[4*(q+v)+i]] for i in range(f)]
        output=[];di=0
        for track in channels:
            width=track['width']
            if track['constant']:
                prefix={4:'Quat',3:'Vec3',1:'Float'}[width]
                low=fields[prefix+'Min'];scale=fields[prefix+'Max']-low
                value=[low+scale*palette[i] for i in track['indices']]
            else:value=dynamic[di];di+=1
            if width==4:
                norm=math.sqrt(sum(x*x for x in value))
                if not math.isfinite(norm) or norm<1e-8:raise ValueError('Invalid assembled quaternion')
                value=[x/norm for x in value]
            if not all(math.isfinite(x) for x in value):raise ValueError('Non-finite assembled track')
            output.append(value)
        frames.append(output)
    return times,frames
