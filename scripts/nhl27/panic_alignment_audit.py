"""Recover skating alignment configurations and their exact asset dependencies.

Reads hash-checked source payloads and original native banks. Does not install
or qualify executable behavior merely because an asset identity is present.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
from ant_tags import LAYOUTS, compile_tree
from deke_closure_stage import closure
from deke_gesture_audit import Source
from deke_native_audit import native_id
from native_ant import Bank


def run(root):
    source=Source(root)
    descriptor=next(t for t in source.types if t['name_candidates']==['SkatingFeatureAsset'])
    configs=[];roots=set()
    for entry in source.catalog:
        if descriptor['guid'] not in entry.get('fixup',{}).get('types',{}):continue
        document=source.partition(entry['fixup']['partition'])
        for node in document['instances']:
            if node['type']!=['SkatingFeatureAsset']:continue
            fields={k:node['fields'][k] for k in ('PathLock','WorldLocatorParam','LocatorAlignmentSolver')}
            configs.append(dict(name=entry['name'],sha1=entry['asset']['sha1'],
                partition=entry['fixup']['partition'],offset=node['offset'],fields=fields))
            for name in ('WorldLocatorParam','LocatorAlignmentSolver'):
                if fields[name]:roots.add(fields[name]['external']['partition'])
    banks=[Bank((root/'ant/nhl24_160'/f'{name}.bin').read_bytes()) for name in ('state','launch')]
    occupied=set().union(*(bank.by_id for bank in banks))
    graph=closure(source.catalog,roots,occupied)
    if graph['missing'] or graph['ambiguous']:raise ValueError('Incomplete alignment dependency closure')
    output=root/'deke-trial/panic-alignment';output.mkdir(exist_ok=True)
    types=Counter();documents=[];fragments=[]
    for entry in graph['nodes']:
        part=entry['fixup']['partition'];document=source.partition(part)
        documents.append(dict(name=entry['name'],fields=document))
        types.update(kind for node in document['instances'] for kind in node['type'])
        # Only the locator-parameter class is newly qualified by this audit.
        # Other supported classes remain in the closure until solver adapters
        # can supply their consumer semantics and complete interface fixups.
        if len(document['instances'])!=1 or document['instances'][0]['type']!=['ScenarioLocatorParamsAsset']:continue
        ids={offset:native_id(guid) for guid,offset in document['exported_instance_guids'].items()}
        fragment=compile_tree(document,dict(internal_offset=document['instances'][0]['offset']),occupied,ids)
        raw=fragment.pop('data');path=output/f'{part}.bin';path.write_bytes(raw)
        fragment.update(path=str(path),sha256=hashlib.sha256(raw).hexdigest())
        fragment.setdefault('compatibility_requirements',[]).append(dict(kind='panic_alignment_runtime_required'))
        fragments.append(fragment)
    report=dict(status='alignment_dependencies_not_integrated',installed=False,configurations=configs,
        closure=graph,fields=documents,evidence=source.evidence,types=dict(types),
        unsupported_types=sorted(set(types)-set(LAYOUTS)),locator_fragments=fragments,
        native_registry=dict(property='0xac8fc18',class_hash='0x738a4434',constructor='0x8c7130',
            remove='0x8c55d0',bytes=0x770),
        feature_allocations=dict(source=0x650,target=0x70,source_locator_offset=0x640,
            target_locator_offset=None),
        unresolved=['Native skating-feature storage and lifecycle integration',
                    'Alignment solver and handler runtime adapters',
                    'Locator producer scheduling and active graph integration'])
    (output/'report.json').write_text(json.dumps(report,indent=2))
    return report


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    report=run(parser.parse_args().root)
    print(json.dumps(dict(status=report['status'],configurations=len(report['configurations']),
        dependencies=len(report['closure']['nodes']),types=report['types'],
        staged_locator_parameters=len(report['locator_fragments']))))
