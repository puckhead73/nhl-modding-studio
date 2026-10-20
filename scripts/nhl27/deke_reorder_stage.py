"""Stage lossless channel-order conversions for existing NHL24 clip identities.

Requires extracted game and launch banks for both target versions. Emits resources
and a qualification report only; it does not change native banks or installations.
Usage: python scripts/nhl27/deke_reorder_stage.py COMPATIBILITY_ROOT
"""
import hashlib
import json
import struct
import sys
from pathlib import Path
from animation_dct import decode, envelope, take
from animation_fields import Fields
from dct_reorder import permutation, reorder
from native_ant import Bank


def run(root,bank_name='state'):
    suffix='' if bank_name=='state' else bank_name+'-'
    dest=root/'deke-trial';out=dest/(suffix+'reordered');out.mkdir(exist_ok=True)
    audit=json.loads((dest/(suffix+'native-comparison.json')).read_text())
    stage=json.loads((dest/'stage-report.json').read_text())
    types=json.loads((root/'source-type-descriptors.json').read_text())
    bindings={x['partition']:x for x in stage['bindings']};report=[]
    for build in audit['banks']:
        launch=Bank((root/'ant'/build['build']/'launch.bin').read_bytes())
        result=dict(build=build['build'],launch_sha256=hashlib.sha256(launch.data).hexdigest(),clips=[])
        for clip in build['clips']:
            row=dict(name=clip['name'],source_sha1=clip['source_sha1'])
            try:
                if not clip['status'].startswith('decoded'):raise ValueError('Native control was rejected')
                fields=next(x['fields'] for x in clip['source_fields']['instances'] if 'DctAnimationAsset' in x['type'])
                binding=bindings[fields['ChannelToDofAsset']['external']['partition']]
                bfields=Fields(Path(binding['payload']).read_bytes(),types).export()
                source=next(x['fields']['0xec5cc5cd'] for x in bfields['instances'] if '0xec5cc5cd' in x['fields'])
                link=next(x for x in clip['bindings'] if x['field']=='ChannelToDofAsset')
                target_object=launch.by_id[int(link['target_id'],16)]
                if target_object['size']!=32 or target_object['type_hash']!=924757995:
                    raise ValueError('Unqualified channel-to-DOF native type')
                offset=target_object['offset'];n=struct.unpack_from('<I',launch.blob,offset+24)[0]
                if n>16384:raise ValueError('Channel binding exceeds bound')
                target=list(struct.unpack('<'+'I'*n,take(launch.blob,launch.pointer(offset+16),n*4)))
                data=(dest/(clip['source_sha1']+'.resource')).read_bytes()
                meta,frames,_=decode(data);tm=clip['target_metadata']
                dims=('quaternions','vectors','float_vectors')
                if any(meta[x]!=tm[x] for x in dims):raise ValueError('Target channel dimensions differ')
                order=permutation(source,target,*(meta[x] for x in dims))
                # Identity round trip detects serializer changes before conversion.
                if reorder(data,list(range(len(order))))!=data:raise ValueError('Identity repack not byte-exact')
                converted=reorder(data,order);cm,cf,_=decode(converted)
                if cf!=[[f[i] for i in order] for f in frames]:raise ValueError('Decoded components changed')
                path=out/(build['build']+'-'+clip['source_sha1']+'.resource');path.write_bytes(converted)
                env,body=envelope(converted);old=tm['envelope']
                row.update(status='lossless_reorder_verified_not_installed',output=str(path),
                    sha256=hashlib.sha256(converted).hexdigest(),channel_order=order,
                    samples=cm['keys'],channel_count=len(order),source_dofs=source,target_dofs=target,
                    exact_decoded_component_match=True,identity_repack_byte_exact=True,
                    body_size=len(body),target_body_capacity=old['body_size'],
                    key_count=len(env['key_times']),target_key_capacity=len(old['key_times']),
                    fits_existing_buffers=len(body)<=old['body_size'] and len(env['key_times'])<=len(old['key_times']),
                    same_dof_set_id=next(x['same_id'] for x in clip['bindings'] if x['field']=='DofSetList'))
            except (ValueError,KeyError,StopIteration) as exc:row.update(status='rejected',error=str(exc))
            result['clips'].append(row)
        report.append(result)
    (dest/(suffix+'reorder-report.json')).write_text(json.dumps(report,indent=2))
    print(json.dumps([dict(build=b['build'],converted=sum('output' in x for x in b['clips']),
        fits_existing_buffers=sum(x.get('fits_existing_buffers',False) for x in b['clips']),
        rejections=[dict(name=x['name'],error=x['error']) for x in b['clips'] if 'error' in x]) for b in report],indent=2))


if __name__=='__main__':run(Path(sys.argv[1]),sys.argv[2] if len(sys.argv)>2 else 'state')
