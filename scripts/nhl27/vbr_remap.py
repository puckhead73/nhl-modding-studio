"""Losslessly permute VBR constant/dynamic tracks into a target DOF order."""
import copy
import json
import struct
import sys
from pathlib import Path
from vbr_decode_audit import layout
from vbr_coefficients import decode,encode
from vbr_stage import envelope


def tracks(fields,parts):
    kinds=[4]*(fields['QuaternionCount']+fields['ConstQuaternionCount'])
    kinds += [3]*(fields['Vector3Count']+fields['ConstVector3Count'])
    kinds += [1]*(fields['FloatCount']+fields['ConstFloatCount'])
    constant=[bool(i%2) for i,n in enumerate(parts['runs']) for _ in range(n)]
    if len(kinds)!=len(constant):raise ValueError('Channel map length mismatch')
    blocks=[decode(parts['widths'],b) for b in parts['blocks']]
    result=[];ci=di=0
    for width,is_const in zip(kinds,constant):
        if is_const:
            result.append(dict(width=width,constant=True,indices=parts['indices'][ci:ci+width]));ci+=width
        else:
            result.append(dict(width=width,constant=False,widths=parts['widths'][di:di+width],
                               blocks=[b[di:di+width] for b in blocks]));di+=width
    if ci!=len(parts['indices']) or di!=len(parts['widths']):raise ValueError('Channel category counts differ')
    return result


def remap(fields,data,source_dofs,target_dofs):
    if len(set(source_dofs))!=len(source_dofs) or len(set(target_dofs))!=len(target_dofs):
        raise ValueError('Ambiguous DOF identity')
    if set(source_dofs)!=set(target_dofs):raise ValueError('DOF sets differ')
    parts=layout(fields,data);original=tracks(fields,parts)
    if len(original)!=len(source_dofs):raise ValueError('DOF count differs from channels')
    order=[source_dofs.index(x) for x in target_dofs];selected=[original[i] for i in order]
    if [x['width'] for x in selected]!=[x['width'] for x in original]:raise ValueError('Cross-category DOF permutation')
    # The first vector may use a distinct trajectory range. Do not silently change it.
    if (fields['TrajMin'],fields['TrajMax'])!=(fields['Vec3Min'],fields['Vec3Max']):
        first=next(i for i,t in enumerate(original) if t['width']==3)
        if order[first]!=first:raise ValueError('Trajectory track moved across distinct quantization ranges')
    runs=[0];kind=False
    for t in selected:
        if t['constant']!=kind:runs.append(0);kind=not kind
        runs[-1]+=1
    if max(runs)>255:raise ValueError('Channel run needs extended encoding')
    indices=[i for t in selected if t['constant'] for i in t['indices']]
    widths=[w for t in selected if not t['constant'] for w in t['widths']]
    blocks=[]
    for i,old in enumerate(parts['blocks']):
        channels=[c for t in selected if not t['constant'] for c in t['blocks'][i]]
        blocks.append(encode(widths,channels,old[:3]))
    if any(len(b)>65535 for b in blocks):raise ValueError('VBR block size overflow')
    index_type='B' if fields['ConstPaletteSize']<=256 else 'H'
    stream=bytes(parts['times'])+struct.pack('<'+index_type*len(indices),*indices)+bytes(runs)
    stream+=struct.pack('<'+'I'*len(widths),*widths)+b''.join(blocks)+bytes(8)
    palette=parts['palette'];keys=struct.pack('<'+'H'*len(blocks),*(len(b) for b in blocks))
    output=struct.pack('<12I',len(palette)//4,48,0,len(blocks),48+len(palette),0,len(stream),48+len(palette)+len(keys),0,0,0,0)
    output+=palette+keys+stream;output+=bytes((-len(output))%16)+struct.pack('<3I',4,16,28)
    revised=copy.deepcopy(fields);revised['ConstChanMapSize']=len(runs)
    actual=tracks(revised,layout(revised,output))
    if actual!=selected:raise ValueError('Reconstructed tracks differ after permutation')
    return revised,output,order


def run(root):
    folder=root/'deke-trial/vbr';rows=json.loads((folder/'stage.json').read_text())
    bindings={x['prefix']:x for x in json.loads((folder/'bindings.json').read_text())};report=[]
    for row in rows:
        prefix=Path(row['prefix']);f=row['fields']['instances'][1]['fields'];binding=bindings[row['prefix']]
        data=prefix.with_suffix('.resource').read_bytes()
        identity,identical,_=remap(f,data,binding['source_dofs'],binding['source_dofs'])
        if identity!=f or identical!=data:raise ValueError('Identity VBR remap differs')
        fields,converted,order=remap(f,data,binding['source_dofs'],binding['target_dofs'])
        inverse_fields,inverted,_=remap(fields,converted,binding['target_dofs'],binding['source_dofs'])
        if inverse_fields!=f or inverted!=data:raise ValueError('Inverse VBR remap differs')
        prefix.with_suffix('.remapped.resource').write_bytes(converted)
        prefix.with_suffix('.remapped.fields.json').write_text(json.dumps(fields,indent=2))
        report.append(dict(name=row['name'],prefix=str(prefix),order=order,identity_exact=True,inverse_exact=True,
                           tracks_exact=True,size=len(converted),envelope=envelope(converted)[0]))
    (folder/'remap-report.json').write_text(json.dumps(report,indent=2));print(json.dumps(dict(remapped=len(report))))


if __name__=='__main__':run(Path(sys.argv[1]))
