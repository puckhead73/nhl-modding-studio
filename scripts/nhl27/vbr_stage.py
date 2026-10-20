"""Validate VBR envelopes and stage exact source fields without claiming motion decode."""
import hashlib
import json
import math
import struct
import sys
from pathlib import Path
from animation_dct import take
from animation_fields import Fields
from compatibility import Install, Oodle, frames
from deke_native_audit import native_id
from native_ant import Bank


def envelope(data):
    h=struct.unpack('<12I',take(data,0,48))
    np,palette,z1,nk,keys,z2,nb,body,z3,z4,z5,z6=h
    if any((z1,z2,z3,z4,z5,z6)) or palette!=48 or keys!=palette+np*4 or body!=keys+nk*2:
        raise ValueError('Unsupported VBR envelope layout')
    pal=take(data,palette,np*4);kt=take(data,keys,nk*2);stream=take(data,body,nb)
    padding=(-(body+nb))%16
    if data[body+nb:]!=bytes(padding)+struct.pack('<III',4,16,28):
        raise ValueError('Unsupported VBR resource trailer')
    if not all(math.isfinite(x[0]) for x in struct.iter_unpack('<f',pal)):
        raise ValueError('Non-finite VBR palette')
    return dict(palette_count=np,key_word_count=nk,stream_size=nb,
                palette_offset=palette,key_offset=keys,stream_offset=body),pal,kt,stream


def run(root):
    dest=root/'deke-trial'/'vbr';dest.mkdir(exist_ok=True)
    stage=json.loads((root/'deke-trial/stage-report.json').read_text())
    types=json.loads((root/'source-type-descriptors.json').read_text())
    ins=Install('nhl27');oodle=Oodle();inventories={};rows=[]
    banks={b:Bank((root/'ant'/b/'state.bin').read_bytes()) for b in ['nhl24_base','nhl24_160']}
    for row in stage['clips']:
        if row['status']!='rejected':continue
        toc=row['layer']
        if toc not in inventories:
            inv=json.loads((root/('nhl27-'+toc.replace('/','_')+'.json')).read_text())
            inventories[toc]={(a['kind'],a['name']):a for b in inv['bundles'] for a in b['assets']}
        assets=inventories[toc];item=dict(name=row['name'],layer=toc)
        try:
            decoded={}
            for kind in ['ebx','res']:
                a=assets[kind,row['name']];raw=ins.read(a['span']);_,data=frames(raw,oodle)
                if hashlib.sha1(raw).hexdigest()!=a['sha1'] or data is None or len(data)!=a['size']:
                    raise ValueError('Source occurrence hash/size mismatch')
                decoded[kind]=data
            fields=Fields(decoded['ebx'],types).export()
            vf=next(x['fields'] for x in fields['instances'] if 'VbrAnimationAsset' in x['type'])
            if vf['CodecType']!=0x56425220 or vf['Resource']!=int(assets['res',row['name']]['resource_id'],16):
                raise ValueError('VBR metadata/resource mismatch')
            env,_,_,_=envelope(decoded['res'])
            if env['palette_count']!=vf['ConstPaletteSize']:raise ValueError('Palette metadata mismatch')
            ident=native_id(fields['partition']);matches={}
            for build,bank in banks.items():
                obj=bank.by_id.get(ident)
                if obj is None:continue
                at=bank.pointer(obj['offset']+32)
                matches[build]=dict(controller=obj,animation=next(o for o in bank.by_id.values() if o['offset']==at))
            prefix=dest/assets['res',row['name']]['sha1']
            prefix.with_suffix('.ebx').write_bytes(decoded['ebx']);prefix.with_suffix('.resource').write_bytes(decoded['res'])
            prefix.with_suffix('.fields.json').write_text(json.dumps(fields,indent=2))
            item.update(status='envelope_and_fields_validated_motion_not_decoded',fields=fields,envelope=env,
                sha256=hashlib.sha256(decoded['res']).hexdigest(),prefix=str(prefix),target_matches=matches)
        except (ValueError,KeyError,StopIteration) as exc:item.update(status='rejected',error=str(exc))
        rows.append(item)
    (dest/'stage.json').write_text(json.dumps(rows,indent=2))
    print(json.dumps(dict(variants=len(rows),validated=sum('prefix'in x for x in rows),
        target_matches=sum(bool(x.get('target_matches')) for x in rows),errors=[x for x in rows if 'error'in x]),indent=2))


if __name__=='__main__':run(Path(sys.argv[1]))
