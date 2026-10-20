"""Compare staged source DCT clips against NHL24 native controls, without writes to games.

Usage: python scripts/nhl27/deke_native_audit.py COMPATIBILITY_ROOT
Reports identifier correspondence separately from binding and behavior compatibility.
"""
import hashlib
import json
import struct
import sys
import uuid
from pathlib import Path
from animation_dct import decode
from animation_fields import Fields
from native_ant import Bank


def native_id(guid):
    return int.from_bytes(uuid.UUID(guid).bytes_le[:8], 'little')


def run(root, bank_name='state'):
    dest = root / 'deke-trial'
    staged = json.loads((dest / 'stage-report.json').read_text())
    types = json.loads((root / 'source-type-descriptors.json').read_text())
    report = {'status': 'inspection_only_not_installable', 'banks': []}
    for build in ('nhl24_base', 'nhl24_160'):
        data = (root / 'ant' / build / (bank_name+'.bin')).read_bytes()
        bank = Bank(data)
        result = dict(build=build, sha256=hashlib.sha256(data).hexdigest(),
                      objects=len(bank.objects), blob_offset=bank.sections['blob'][0],
                      blob_size=len(bank.blob), clips=[])
        # Build once: external entries identify bindings outside this serialized bank.
        external = {x[1]: x for x in bank.external}
        internal_ids = {o['offset']:ident for ident,o in bank.by_id.items()}
        for source in staged['clips']:
            if 'partition' not in source:
                continue
            ident = native_id(source['partition'])
            if ident not in bank.by_id:
                continue
            row = dict(name=source['name'], source_sha1=source['resource_sha1'],
                       root_id=f'0x{ident:016x}')
            try:
                controller, animation, resource = bank.clip(ident)
                meta, _, checks = decode(resource)
                fields = Fields((dest / (source['resource_sha1'] + '.ebx')).read_bytes(), types).export()
                dct = next(x['fields'] for x in fields['instances'] if 'DctAnimationAsset' in x['type'])
                bindings = []
                for name, offset in [('ChannelToDofAsset', 40), ('DofSetList', 48)]:
                    source_id = native_id(dct[name]['external']['instance'])
                    entry = external.get(animation['offset'] + offset)
                    target_id = entry[2] if entry else internal_ids.get(bank.reloc.get(animation['offset']+offset))
                    bindings.append(dict(field=name, source_id=f'0x{source_id:016x}',
                        target_id=f'0x{target_id:016x}' if target_id is not None else None,
                        same_id=source_id == target_id,
                        target_in_this_bank=target_id in bank.by_id))
                row.update(status='decoded_control_binding_unqualified', controller=controller,
                           animation=animation, target_metadata=meta, target_checks=checks,
                           source_metadata=source['metadata'], bindings=bindings,
                           source_fields=fields,
                           target_controller_scalars=list(struct.unpack_from('<4f6B', bank.blob, controller['offset']+48)),
                           target_resource_sha256=hashlib.sha256(resource).hexdigest())
            except (ValueError, KeyError, StopIteration) as exc:
                row.update(status='rejected', error=str(exc))
            result['clips'].append(row)
        report['banks'].append(result)
    (dest / ('native-comparison.json' if bank_name=='state' else bank_name+'-native-comparison.json')).write_text(json.dumps(report, indent=2))
    print(json.dumps([dict(build=b['build'], matched=len(b['clips']),
        decoded=sum(x['status'].startswith('decoded') for x in b['clips']),
        differing_channel_map_ids=sum(any(y['field']=='ChannelToDofAsset' and not y['same_id']
            for y in x.get('bindings', [])) for x in b['clips'])) for b in report['banks']], indent=2))


if __name__ == '__main__':
    run(Path(sys.argv[1]),sys.argv[2] if len(sys.argv)>2 else 'state')
