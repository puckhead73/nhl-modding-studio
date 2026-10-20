"""Stage panic runtime signals outside the animation import closure."""
import argparse
import hashlib
import json
from pathlib import Path
from animation_fields import Fields
from ant_controller_stage import exported_id_map
from ant_tags import compile_tree
from native_ant import Bank


SIGNALS = (
    ('a60c82f883a1aa5a7e5e28c80b86c7482023ccad', 0xa04e5c3762bb2993,
     'FloatAsset', 'panic closest player distance'),
    ('72e994f4ec84b0def9f7d551f6c4e1a4b40ee5a4', 0x9238092cfdec447c,
     'BoolAsset', 'panic alignment request'),
    ('bf0193593de288af87f308ab6c1043e249e87019', 0x9f2ed2b822171119,
     'BoolAsset', 'PTM skating node active'),
)


def run(root):
    trial=root/'deke-trial'
    directory=trial/'panic-bindings'
    types=json.loads((root/'source-type-descriptors.json').read_text())
    occupied=set()
    for name in ('state','launch'):
        occupied.update(Bank((root/'ant/nhl24_160'/f'{name}.bin').read_bytes()).by_id)
    controllers=json.loads((trial/'controller-fragments/report.json').read_text())
    for row in controllers['partitions']:
        occupied.update(o['id'] for o in row.get('fragment',{}).get('objects',[]))
    rows=[]
    for sha1,ident,kind,name in SIGNALS:
        path=directory/f'{sha1}.ebx'
        reader=Fields(path.read_bytes(),types)
        ids=exported_id_map(reader)
        fields=reader.export()
        if len(ids)!=1 or next(iter(ids.values()))!=ident or fields['instances'][0]['type']!=[kind]:
            raise ValueError('Unexpected panic signal identity or type')
        if ident in occupied:
            raise ValueError('Runtime signal already defined; do not duplicate its context storage')
        occupied.add(ident)
        fragment=compile_tree(fields,dict(internal_offset=fields['instances'][0]['offset']),occupied,ids)
        data=fragment.pop('data')
        output=directory/f'{sha1}.bin'
        output.write_bytes(data)
        fragment.update(path=str(output),sha256=hashlib.sha256(data).hexdigest(),
            source_ebx_sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        fragment.setdefault('compatibility_requirements',[]).append(dict(
            kind='panic_runtime_producer_required',source_id=hex(ident)))
        rows.append(dict(name=name,fragment=fragment))
    report=dict(installed=False,status='staged_runtime_signals_not_installable',fragments=rows)
    (trial/'panic-runtime-fragments.json').write_text(json.dumps(report,indent=2))
    return report


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    report=run(parser.parse_args().root)
    print(json.dumps(dict(status=report['status'],signals=len(report['fragments']))))
